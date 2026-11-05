// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Thin adapter between the pure core, the Windows services and the webviews.
//!
//! ```text
//!  sheen-win services ─┐                      ┌─► island webview (sheen://presentation)
//!  UI commands ────────┼─► Signal channel ─► reducer thread (sheen_core::Island)
//!  hit-tester (hover) ─┘                      └─► HitRegion ─► hit-tester thread
//!
//!  sheen-player ─► sheen://player ─► main window (created on demand)
//!       └────────► SMTC publisher ─► Windows media controls ─► island
//! ```

mod hit_test;
mod music;
mod settings;
mod tray;

use music::Music;
use settings::{Settings, ThemePref};
use sheen_core::{
    HitRegion, Island, IslandConfig, MediaCommand, Presentation, SheenEvent, CANVAS_WIDTH,
};
use sheen_win::{Services, Signal};
use std::path::PathBuf;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::webview::Color;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, State, Theme, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

const EV_PRESENTATION: &str = "sheen://presentation";
const EV_ARTWORK: &str = "sheen://artwork";
const EV_SETTINGS: &str = "sheen://settings";
/// Upper bound for how long the UI's spring needs to settle (see `HitRegion`).
const TRANSITION: Duration = Duration::from_millis(450);
const MAIN_WINDOW: &str = "main";

/// Shared state for Tauri commands.
pub struct Shared {
    signals: Sender<Signal>,
    services: Services,
    presentation: Mutex<Presentation>,
    artwork: Mutex<Option<String>>,
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    music: Arc<Music>,
}

impl Shared {
    /// Mutate, persist and broadcast the settings.
    pub fn update_settings(&self, app: &AppHandle, f: impl FnOnce(&mut Settings)) {
        let snapshot = {
            let mut settings = self.settings.lock().unwrap();
            let before = serde_json::to_value(&*settings).ok();
            f(&mut settings);
            if serde_json::to_value(&*settings).ok() == before {
                return;
            }
            settings.save(&self.settings_path);
            settings.clone()
        };
        app.set_theme(snapshot.theme.as_tauri());
        let _ = app.emit(EV_SETTINGS, snapshot);
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            current_presentation,
            current_artwork,
            expand,
            collapse,
            media_command,
            adjust_volume,
            get_settings,
            set_theme,
            open_main_window,
            music::library_tracks,
            music::library_status,
            music::rescan_library,
            music::choose_music_folder,
            music::play_tracks,
            music::player_command,
            music::save_player_volume,
            music::player_state,
            music::player_cover,
        ])
        .setup(|app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let mut settings = Settings::load(&settings_path);
            if settings.music_dir.is_none() {
                settings.music_dir = app.path().audio_dir().ok().filter(|d| d.is_dir());
                settings.save(&settings_path);
            }
            app.set_theme(settings.theme.as_tauri());

            let window = app.get_webview_window("island").expect("island window");
            prepare_window(&window)?;

            #[cfg(windows)]
            let hwnd = window.hwnd()?.0 as isize;
            #[cfg(not(windows))]
            let hwnd = 0isize;
            let music = Music::start(app.handle(), hwnd, settings.player_volume);
            if let Some(dir) = settings.music_dir.clone() {
                music.rescan(app.handle(), dir);
            }

            let (tx, rx) = channel::<Signal>();
            let island = Island::new(IslandConfig::default());
            let shared = Arc::new(Shared {
                signals: tx.clone(),
                services: Services::start(tx.clone()),
                presentation: Mutex::new(island.present()),
                artwork: Mutex::new(None),
                settings: Mutex::new(settings),
                settings_path,
                music,
            });
            app.manage(shared.clone());

            let region = Arc::new(Mutex::new(HitRegion::new(TRANSITION)));
            region
                .lock()
                .unwrap()
                .set_target(island.present().geometry.rect(), Instant::now());
            hit_test::spawn(window.clone(), region.clone(), tx);

            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("sheen-reducer".into())
                .spawn(move || reducer(island, rx, handle, shared, region))?;

            tray::create(app.handle())?;
            window.show()?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Sheen")
        .run(|_app, event| {
            // Closing the main window destroys it (freeing its webview); the
            // app itself keeps running in the island and the tray.
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}

/// Pin the canvas to the top-centre of the primary monitor and apply the
/// no-activate / tool-window styles. The window never moves or resizes again.
fn prepare_window(window: &WebviewWindow) -> tauri::Result<()> {
    if let Some(monitor) = window.primary_monitor()? {
        let scale = monitor.scale_factor();
        let width = (CANVAS_WIDTH * scale).round() as i32;
        let x = monitor.position().x + (monitor.size().width as i32 - width) / 2;
        window.set_position(PhysicalPosition::new(x, monitor.position().y))?;
    }
    #[cfg(windows)]
    if let Err(e) = sheen_win::apply_island_window_style(window.hwnd()?.0 as isize) {
        log::warn!("could not apply island window style: {e}");
    }
    // Start fully click-through; the hit-tester opts the capsule back in.
    window.set_ignore_cursor_events(true)?;
    Ok(())
}

/// Show the library/settings window, creating it on first use. It is
/// destroyed again when closed so its webview costs nothing while unused.
pub fn show_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    let theme = app.state::<Arc<Shared>>().settings.lock().unwrap().theme;
    let dark = match theme {
        ThemePref::Dark => true,
        ThemePref::Light => false,
        ThemePref::System => app
            .get_webview_window("island")
            .and_then(|w| w.theme().ok())
            .is_some_and(|t| t == Theme::Dark),
    };
    // Paint the right background before the page loads: no white flash.
    let background = if dark {
        Color(28, 28, 30, 255)
    } else {
        Color(246, 246, 248, 255)
    };
    WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::App("main.html".into()))
        .title("Sheen")
        .inner_size(900.0, 600.0)
        .min_inner_size(640.0, 420.0)
        .center()
        .theme(theme.as_tauri())
        .background_color(background)
        .build()?;
    Ok(())
}

/// Owns the `Island`; the only place state transitions happen.
fn reducer(
    mut island: Island,
    rx: std::sync::mpsc::Receiver<Signal>,
    app: AppHandle,
    shared: Arc<Shared>,
    region: Arc<Mutex<HitRegion>>,
) {
    loop {
        let timeout = island
            .next_deadline()
            .map(|d| d.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::from_secs(3600));

        let changed = match rx.recv_timeout(timeout) {
            Ok(Signal::Event(event)) => island.handle(event, Instant::now()),
            Ok(Signal::Artwork(url)) => {
                *shared.artwork.lock().unwrap() = url.clone();
                let _ = app.emit_to("island", EV_ARTWORK, url);
                false
            }
            Err(RecvTimeoutError::Timeout) => island.tick(Instant::now()),
            Err(RecvTimeoutError::Disconnected) => return,
        };

        if changed {
            let presentation = island.present();
            region
                .lock()
                .unwrap()
                .set_target(presentation.geometry.rect(), Instant::now());
            *shared.presentation.lock().unwrap() = presentation.clone();
            let _ = app.emit_to("island", EV_PRESENTATION, presentation);
        }
    }
}

type Ctx<'a> = State<'a, Arc<Shared>>;

#[tauri::command]
fn current_presentation(state: Ctx) -> Presentation {
    state.presentation.lock().unwrap().clone()
}

#[tauri::command]
fn current_artwork(state: Ctx) -> Option<String> {
    state.artwork.lock().unwrap().clone()
}

#[tauri::command]
fn expand(state: Ctx) {
    let _ = state
        .signals
        .send(Signal::Event(SheenEvent::ExpandRequested));
}

#[tauri::command]
fn collapse(state: Ctx) {
    let _ = state
        .signals
        .send(Signal::Event(SheenEvent::CollapseRequested));
}

#[tauri::command]
fn media_command(state: Ctx, command: MediaCommand) {
    state.services.media_command(command);
}

#[tauri::command]
fn adjust_volume(state: Ctx, delta: f32) {
    state.services.adjust_volume(delta.clamp(-1.0, 1.0));
}

#[tauri::command]
fn get_settings(state: Ctx) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn set_theme(app: AppHandle, state: Ctx, theme: ThemePref) {
    state.update_settings(&app, |s| s.theme = theme);
}

#[tauri::command]
fn open_main_window(app: AppHandle) {
    // Window creation must not block the IPC thread on Windows.
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Err(e) = show_main_window(&handle) {
            log::error!("could not open main window: {e}");
        }
    });
}
