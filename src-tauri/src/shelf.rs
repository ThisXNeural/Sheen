//! The shelf: a temporary parking spot for files, like Dropover on macOS.
//! Items are references to files on disk, not copies.

use crate::Shared;
use sheen_core::{SheenEvent, ShelfItem};
use sheen_win::{DropOutcome, Signal};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;

const MAX_ITEMS: usize = 60;
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png"];
const MAX_THUMB_SOURCE: u64 = 40 * 1024 * 1024;

#[derive(Default)]
pub struct Shelf {
    items: Vec<ShelfItem>,
    next_id: u64,
}

impl Shelf {
    /// Add files, ignoring ones already on the shelf. Returns `true` if
    /// anything was added.
    pub fn add(&mut self, paths: &[PathBuf]) -> bool {
        let mut changed = false;
        for path in paths {
            let p = path.to_string_lossy().into_owned();
            if self.items.iter().any(|i| i.path == p) {
                continue;
            }
            self.next_id += 1;
            self.items.push(item(self.next_id, path));
            changed = true;
        }
        if self.items.len() > MAX_ITEMS {
            let excess = self.items.len() - MAX_ITEMS;
            self.items.drain(..excess);
        }
        changed
    }

    pub fn remove(&mut self, ids: &[u64]) {
        self.items.retain(|i| !ids.contains(&i.id));
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn items(&self) -> Vec<ShelfItem> {
        self.items.clone()
    }

    pub fn get(&self, id: u64) -> Option<&ShelfItem> {
        self.items.iter().find(|i| i.id == id)
    }

    pub fn paths(&self, ids: &[u64]) -> Vec<PathBuf> {
        self.items.iter().filter(|i| ids.contains(&i.id)).map(|i| PathBuf::from(&i.path)).collect()
    }
}

fn item(id: u64, path: &Path) -> ShelfItem {
    let meta = std::fs::metadata(path).ok();
    let ext =
        path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let kind = if meta.as_ref().is_some_and(|m| m.is_dir()) {
        "folder"
    } else if IMAGE_EXTS.contains(&ext.as_str()) {
        "image"
    } else {
        "file"
    };
    ShelfItem {
        id,
        path: path.to_string_lossy().into_owned(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned()),
        kind: kind.into(),
        ext,
        size: meta.filter(|m| m.is_file()).map(|m| m.len()),
    }
}

impl Shared {
    pub fn shelf_changed(&self) {
        let items = self.shelf.lock().unwrap().items();
        let _ = self.signals.send(Signal::Event(SheenEvent::Shelf(items)));
    }

    pub fn shelf_add(&self, paths: &[PathBuf]) {
        if self.shelf.lock().unwrap().add(paths) {
            self.shelf_changed();
        }
    }

    pub fn shelf_thumbnail(&self, id: u64, px: u32) -> Option<Vec<u8>> {
        let item = self.shelf.lock().unwrap().get(id)?.clone();
        if item.kind != "image" || item.size.is_none_or(|s| s > MAX_THUMB_SOURCE) {
            return None;
        }
        sheen_player::art::thumbnail_jpeg(&std::fs::read(&item.path).ok()?, px)
    }
}

// ---- commands ---------------------------------------------------------------

type Ctx<'a> = tauri::State<'a, Arc<Shared>>;

#[tauri::command]
pub fn shelf_remove(state: Ctx, ids: Vec<u64>) {
    state.shelf.lock().unwrap().remove(&ids);
    state.shelf_changed();
}

#[tauri::command]
pub fn shelf_clear(state: Ctx) {
    state.shelf.lock().unwrap().clear();
    state.shelf_changed();
}

#[tauri::command]
pub fn shelf_open(state: Ctx, id: u64) {
    if let Some(item) = state.shelf.lock().unwrap().get(id) {
        sheen_win::open_path(Path::new(&item.path));
    }
}

#[tauri::command]
pub fn shelf_reveal(state: Ctx, id: u64) {
    if let Some(item) = state.shelf.lock().unwrap().get(id) {
        sheen_win::reveal_in_explorer(Path::new(&item.path));
    }
}

#[tauri::command]
pub fn shelf_request(state: Ctx) {
    let _ = state.signals.send(Signal::Event(SheenEvent::ShelfRequested));
}

/// Drag shelf items out to another app. Runs the shell drag loop on the UI
/// thread; items that were dropped somewhere leave the shelf.
#[tauri::command]
pub fn shelf_drag_out(app: AppHandle, state: Ctx, ids: Vec<u64>) {
    let paths = state.shelf.lock().unwrap().paths(&ids);
    if paths.is_empty() {
        return;
    }
    let shared = state.inner().clone();
    let _ = app.run_on_main_thread(move || {
        let outcome = sheen_win::drag_files_out(&paths);
        if outcome != DropOutcome::Cancelled {
            shared.shelf.lock().unwrap().remove(&ids);
            shared.shelf_changed();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupes_and_caps() {
        let mut s = Shelf::default();
        assert!(s.add(&[PathBuf::from("/a.txt"), PathBuf::from("/a.txt")]));
        assert!(!s.add(&[PathBuf::from("/a.txt")]));
        assert_eq!(s.items().len(), 1);
        let many: Vec<PathBuf> = (0..100).map(|i| PathBuf::from(format!("/f{i}.png"))).collect();
        s.add(&many);
        assert_eq!(s.items().len(), MAX_ITEMS);
        assert_eq!(s.items().last().unwrap().name, "f99.png");
        assert_eq!(s.items().last().unwrap().kind, "image");
    }

    #[test]
    fn removes_by_id() {
        let mut s = Shelf::default();
        s.add(&[PathBuf::from("/a"), PathBuf::from("/b")]);
        let first = s.items()[0].id;
        s.remove(&[first]);
        assert_eq!(s.items().len(), 1);
        assert_eq!(s.paths(&[s.items()[0].id]), vec![PathBuf::from("/b")]);
    }
}
