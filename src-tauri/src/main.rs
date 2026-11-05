// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Thin adapter between the pure core, the Windows services and the webview.
//!
//! ```text
//!  sheen-win services ─┐                      ┌─► webview (sheen://presentation)
//!  UI commands ────────┼─► Signal channel ─► reducer thread (sheen_core::Island)
//!  hit-tester (hover) ─┘                      └─► HitRegion ─► hit-tester thread
//! ```

mod hit_test;

use sheen_core::{
    HitRegion, Island, IslandConfig, MediaCommand, Presentation, SheenEvent, CANVAS_WIDTH,
};
use sheen_win::{Services, Signal};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewWindow};

const EV_PRESENTATION: &str = "sheen://presentation";
const EV_ARTWORK: &str = "sheen://artwork";
/// Upper bound for how long the UI's spring needs to settle (see `HitRegion`).
const TRANSITION: Duration = Duration::from_millis(450);

/// Shared, read-mostly state for Tauri commands.
struct Shared {
    signals: Sender<Signal>,
    services: Services,
    presentation: Mutex<Presentation>,
    artwork: Mutex<Option<String>>,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            current_presentation,
            current_artwork,
            expand,
            collapse,
            media_command,
            adjust_volume,
        ])
        .setup(|app| {
            let window = app.get_webview_window("island").expect("island window");
            prepare_window(&window)?;

            let (tx, rx) = channel::<Signal>();
            let island = Island::new(IslandConfig::default());
            let shared = Arc::new(Shared {
                signals: tx.clone(),
                services: Services::start(tx.clone()),
                presentation: Mutex::new(island.present()),
                artwork: Mutex::new(None),
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

            window.show()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Sheen");
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
                let _ = app.emit(EV_ARTWORK, url);
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
            let _ = app.emit(EV_PRESENTATION, presentation);
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
