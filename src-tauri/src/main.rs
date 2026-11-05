// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Thin adapter between the pure core, the Windows services and the webviews.
//!
//! ```text
//!  sheen-win services ─┐                      ┌─► island webview (sheen://presentation)
//!  UI commands ────────┼─► Signal channel ─► reducer thread (sheen_core::Island)
//!  hit-tester (hover,  │                      └─► HitRegion ─► hit-tester thread
//!   file drags) ───────┘
//!
//!  sheen-player ─► sheen://player ─► main window (created on demand)
//!       └────────► SMTC publisher ─► Windows media controls ─► island
//! ```

mod art;
mod hit_test;
mod music;
mod note;
mod settings;
mod shelf;
mod tray;

use music::Music;
use settings::{Settings, ThemePref};
use sheen_core::{
    HitRegion, Island, IslandConfig, MediaCommand, OsdKind, Presentation, SheenEvent, CANVAS_WIDTH,
};
use sheen_win::{Hotkeys, Services, Signal};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::webview::Color;
use tauri::{
    AppHandle, DragDropEvent, Emitter, Manager, PhysicalPosition, State, Theme, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

const EV_PRESENTATION: &str = "sheen://presentation";
const EV_ARTWORK: &str = "sheen://artwork";
const EV_SETTINGS: &str = "sheen://settings";
/// Upper bound for how long the UI's spring needs to settle (see `HitRegion`).
const TRANSITION: Duration = Duration::from_millis(450);
const MAIN_WINDOW: &str = "main";
const ISLAND_WINDOW: &str = "island";

/// Shared state for Tauri commands.
pub struct Shared {
    signals: Sender<Signal>,
    services: Services,
    hotkeys: Hotkeys,
    presentation: Mutex<Presentation>,
    artwork: Mutex<Option<String>>,
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    music: Arc<Music>,
    shelf: Mutex<shelf::Shelf>,
    note_pinned: AtomicBool,
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
        .register_asynchronous_uri_scheme_protocol(art::SCHEME, art::handle)
        .invoke_handler(tauri::generate_handler![
            current_presentation,
            current_artwork,
            expand,
            collapse,
            media_command,
            adjust_volume,
            get_settings,
            set_theme,
            set_island_option,
            open_main_window,
            music::library_tracks,
            music::library_status,
            music::rescan_library,
            music::choose_music_folder,
            music::play_tracks,
            music::enqueue_tracks,
            music::player_command,
            music::save_player_volume,
            music::player_state,
            music::queue_snapshot,
            music::reveal_track,
            music::collections,
            music::toggle_like,
            music::playlist_create,
            music::playlist_rename,
            music::playlist_delete,
            music::playlist_add,
            music::playlist_remove,
            music::playlist_reorder,
            shelf::shelf_remove,
            shelf::shelf_clear,
            shelf::shelf_open,
            shelf::shelf_reveal,
            shelf::shelf_request,
            shelf::shelf_drag_out,
            note::note_load,
            note::note_save,
            note::note_hidden,
            note::note_toggle,
            note::note_set_pinned,
            note::set_note_shortcut,
        ])
        .setup(|app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let mut settings = Settings::load(&settings_path);
            if settings.music_dir.is_none() {
                settings.music_dir = app.path().audio_dir().ok().filter(|d| d.is_dir());
                settings.save(&settings_path);
            }
            app.set_theme(settings.theme.as_tauri());

            let window = app.get_webview_window(ISLAND_WINDOW).expect("island window");
            prepare_window(&window)?;

            #[cfg(windows)]
            let hwnd = window.hwnd()?.0 as isize;
            #[cfg(not(windows))]
            let hwnd = 0isize;
            let music = Music::start(app.handle(), hwnd, settings.player_volume);
            if let Some(dir) = settings.music_dir.clone() {
                music.rescan(app.handle(), dir);
            }

            let handle = app.handle().clone();
            let hotkeys = Hotkeys::start(move |id| {
                if id == note::HOTKEY_ID {
                    let h = handle.clone();
                    let _ = handle.run_on_main_thread(move || note::toggle(&h));
                }
            });

            let (tx, rx) = channel::<Signal>();
            let island = Island::new(IslandConfig::default());
            let note_shortcut = settings.note_shortcut.clone();
            let shared = Arc::new(Shared {
                signals: tx.clone(),
                services: Services::start(tx.clone()),
                hotkeys,
                presentation: Mutex::new(island.present()),
                artwork: Mutex::new(None),
                settings: Mutex::new(settings),
                settings_path,
                music,
                shelf: Mutex::new(shelf::Shelf::default()),
                note_pinned: AtomicBool::new(false),
            });
            app.manage(shared.clone());
            if let Err(e) = note::apply_shortcut(&shared, &note_shortcut) {
                log::warn!("note shortcut {note_shortcut:?} not registered: {e}");
            }

            // Files dropped on the island land on the shelf.
            let drop_target = shared.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event {
                    drop_target.shelf_add(paths);
                }
            });

            let region = Arc::new(Mutex::new(HitRegion::new(TRANSITION)));
            region.lock().unwrap().set_target(island.present().geometry.rect(), Instant::now());
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
        .run(|app, event| match event {
            // Closing the main window destroys it (freeing its webview); the
            // app itself keeps running in the island and the tray.
            tauri::RunEvent::ExitRequested { code: None, api, .. } => api.prevent_exit(),
            tauri::RunEvent::Exit => {
                if let Some(state) = app.try_state::<Arc<Shared>>() {
                    state.music.flush_session();
                }
            }
            _ => {}
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
            .get_webview_window(ISLAND_WINDOW)
            .and_then(|w| w.theme().ok())
            .is_some_and(|t| t == Theme::Dark),
    };
    // Paint the right background before the page loads: no white flash.
    let background = if dark { Color(28, 28, 30, 255) } else { Color(246, 246, 248, 255) };
    WebviewWindowBuilder::new(app, MAIN_WINDOW, WebviewUrl::App("main.html".into()))
        .title("Sheen")
        .inner_size(1000.0, 660.0)
        .min_inner_size(720.0, 480.0)
        .center()
        .theme(theme.as_tauri())
        .background_color(background)
        // HTML drag and drop inside the library (not files from Explorer).
        .disable_drag_drop_handler()
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
            Ok(Signal::Event(event)) => match filter(&shared, event) {
                Some(event) => island.handle(event, Instant::now()),
                None => false,
            },
            Ok(Signal::Artwork(url)) => {
                *shared.artwork.lock().unwrap() = url.clone();
                let _ = app.emit_to(ISLAND_WINDOW, EV_ARTWORK, url);
                false
            }
            Err(RecvTimeoutError::Timeout) => island.tick(Instant::now()),
            Err(RecvTimeoutError::Disconnected) => return,
        };

        if changed {
            let presentation = island.present();
            region.lock().unwrap().set_target(presentation.geometry.rect(), Instant::now());
            *shared.presentation.lock().unwrap() = presentation.clone();
            let _ = app.emit_to(ISLAND_WINDOW, EV_PRESENTATION, presentation);
        }
    }
}

/// Apply the user's island preferences to incoming events.
fn filter(shared: &Shared, event: SheenEvent) -> Option<SheenEvent> {
    let settings = shared.settings.lock().unwrap();
    match event {
        SheenEvent::Osd(OsdKind::Clipboard { .. }) if !settings.clipboard_peek => None,
        SheenEvent::Transfers(_) if !settings.show_transfers => {
            Some(SheenEvent::Transfers(Vec::new()))
        }
        other => Some(other),
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
    let _ = state.signals.send(Signal::Event(SheenEvent::ExpandRequested));
}

#[tauri::command]
fn collapse(state: Ctx) {
    let _ = state.signals.send(Signal::Event(SheenEvent::CollapseRequested));
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
fn set_island_option(app: AppHandle, state: Ctx, key: String, on: bool) -> Result<(), String> {
    match key.as_str() {
        "clipboardPeek" => state.update_settings(&app, |s| s.clipboard_peek = on),
        "showTransfers" => state.update_settings(&app, |s| s.show_transfers = on),
        _ => return Err(format!("unknown option {key}")),
    }
    Ok(())
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
