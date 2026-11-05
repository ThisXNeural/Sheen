//! The quick note: a small always-on-top card that drops down from the
//! island on a global shortcut (Scroll Lock by default). Text autosaves to
//! `note.txt` in the app data folder.
//!
//! The window is created on first use and then only hidden, so later opens
//! are instant.

use crate::Shared;
use sheen_core::Accelerator;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

pub const LABEL: &str = "note";
pub const HOTKEY_ID: i32 = 1;
const WIDTH: f64 = 360.0;
const HEIGHT: f64 = 320.0;
/// Below the island's resting position.
const TOP: f64 = 50.0;

fn note_path(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap_or_else(|_| std::env::temp_dir()).join("note.txt")
}

/// Show the note, or hide it if it is already in front.
pub fn toggle(app: &AppHandle) {
    match app.get_webview_window(LABEL) {
        Some(w) if w.is_visible().unwrap_or(false) => {
            let _ = w.emit("sheen://note-hide", ());
        }
        Some(w) => show(&w),
        None => match create(app) {
            Ok(w) => show(&w),
            Err(e) => log::error!("could not create note window: {e}"),
        },
    }
}

fn show(w: &WebviewWindow) {
    position(w);
    let _ = w.show();
    let _ = w.set_focus();
    let _ = w.emit("sheen://note-shown", ());
}

fn position(w: &WebviewWindow) {
    if let Ok(Some(monitor)) = w.primary_monitor() {
        let scale = monitor.scale_factor();
        let x = monitor.position().x + ((monitor.size().width as f64 - WIDTH * scale) / 2.0) as i32;
        let y = monitor.position().y + (TOP * scale) as i32;
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
}

fn create(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    let w = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("note.html".into()))
        .title("Sheen note")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .disable_drag_drop_handler()
        .build()?;
    let app2 = app.clone();
    w.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            let pinned = app2.state::<Arc<Shared>>().note_pinned.load(Ordering::Relaxed);
            if !pinned {
                if let Some(w) = app2.get_webview_window(LABEL) {
                    let _ = w.emit("sheen://note-hide", ());
                }
            }
        }
    });
    Ok(w)
}

/// Apply the shortcut from settings. Returns a user-facing error if the
/// combination is invalid or taken.
pub fn apply_shortcut(state: &Shared, shortcut: &str) -> Result<String, String> {
    if shortcut.trim().is_empty() {
        state.hotkeys.set(HOTKEY_ID, None)?;
        return Ok(String::new());
    }
    let acc = Accelerator::parse(shortcut)?;
    state.hotkeys.set(HOTKEY_ID, Some(acc))?;
    Ok(acc.to_string())
}

// ---- commands ---------------------------------------------------------------

type Ctx<'a> = tauri::State<'a, Arc<Shared>>;

#[tauri::command]
pub fn note_load(app: AppHandle) -> String {
    std::fs::read_to_string(note_path(&app)).unwrap_or_default()
}

#[tauri::command]
pub fn note_save(app: AppHandle, text: String) -> Result<(), String> {
    let path = note_path(&app);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}

/// Called by the note page once its closing animation has finished.
#[tauri::command]
pub fn note_hidden(app: AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.hide();
    }
}

#[tauri::command]
pub fn note_toggle(app: AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || toggle(&handle));
}

#[tauri::command]
pub fn note_set_pinned(state: Ctx, pinned: bool) {
    state.note_pinned.store(pinned, Ordering::Relaxed);
}

#[tauri::command]
pub fn set_note_shortcut(app: AppHandle, state: Ctx, shortcut: String) -> Result<String, String> {
    let normalized = apply_shortcut(&state, &shortcut)?;
    state.update_settings(&app, |s| s.note_shortcut = normalized.clone());
    Ok(normalized)
}
