//! Liked songs and playlists. Tracks are referenced by path, so they survive
//! rescans (which renumber track ids).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: u64,
    pub name: String,
    pub tracks: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Collections {
    pub liked: Vec<PathBuf>,
    pub playlists: Vec<Playlist>,
    next_id: u64,
}

impl Collections {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec(self)?)?;
        std::fs::rename(tmp, path)
    }

    /// Returns whether the track is liked afterwards.
    pub fn toggle_like(&mut self, path: &Path) -> bool {
        if let Some(i) = self.liked.iter().position(|p| p == path) {
            self.liked.remove(i);
            false
        } else {
            self.liked.insert(0, path.to_path_buf());
            true
        }
    }

    pub fn create(&mut self, name: &str) -> u64 {
        self.next_id += 1;
        let name = name.trim();
        let name = if name.is_empty() { "New playlist" } else { name };
        self.playlists.push(Playlist { id: self.next_id, name: name.to_string(), tracks: vec![] });
        self.next_id
    }

    pub fn rename(&mut self, id: u64, name: &str) -> bool {
        match self.get_mut(id) {
            Some(p) if !name.trim().is_empty() => {
                p.name = name.trim().to_string();
                true
            }
            _ => false,
        }
    }

    pub fn delete(&mut self, id: u64) -> bool {
        let before = self.playlists.len();
        self.playlists.retain(|p| p.id != id);
        self.playlists.len() != before
    }

    /// Append tracks, skipping ones already in the playlist. Returns how many
    /// were added.
    pub fn add(&mut self, id: u64, paths: &[PathBuf]) -> usize {
        let Some(p) = self.get_mut(id) else { return 0 };
        let mut added = 0;
        for path in paths {
            if !p.tracks.contains(path) {
                p.tracks.push(path.clone());
                added += 1;
            }
        }
        added
    }

    /// Remove entries by position.
    pub fn remove(&mut self, id: u64, positions: &[usize]) {
        if let Some(p) = self.get_mut(id) {
            let mut i = 0;
            p.tracks.retain(|_| {
                let keep = !positions.contains(&i);
                i += 1;
                keep
            });
        }
    }

    /// Move the entry at `from` to `to`.
    pub fn reorder(&mut self, id: u64, from: usize, to: usize) {
        if let Some(p) = self.get_mut(id) {
            if from < p.tracks.len() && to < p.tracks.len() {
                let t = p.tracks.remove(from);
                p.tracks.insert(to, t);
            }
        }
    }

    fn get_mut(&mut self, id: u64) -> Option<&mut Playlist> {
        self.playlists.iter_mut().find(|p| p.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn like_toggles_newest_first() {
        let mut c = Collections::default();
        assert!(c.toggle_like(&p("a")));
        assert!(c.toggle_like(&p("b")));
        assert_eq!(c.liked, vec![p("b"), p("a")]);
        assert!(!c.toggle_like(&p("a")));
        assert_eq!(c.liked, vec![p("b")]);
    }

    #[test]
    fn playlist_lifecycle() {
        let mut c = Collections::default();
        let id = c.create("  Road trip ");
        assert_eq!(c.playlists[0].name, "Road trip");
        assert_eq!(c.add(id, &[p("a"), p("b"), p("a")]), 2);
        assert_eq!(c.add(id, &[p("b"), p("c")]), 1);
        c.reorder(id, 2, 0);
        assert_eq!(c.playlists[0].tracks, vec![p("c"), p("a"), p("b")]);
        c.remove(id, &[1]);
        assert_eq!(c.playlists[0].tracks, vec![p("c"), p("b")]);
        assert!(c.rename(id, "Drive"));
        assert!(!c.rename(id, "   "));
        assert!(c.delete(id));
        assert!(c.playlists.is_empty());
        // Ids are never reused.
        assert_eq!(c.create(""), id + 1);
        assert_eq!(c.playlists[0].name, "New playlist");
    }

    #[test]
    fn roundtrip() {
        let dir = std::env::temp_dir().join(format!("sheen-col-{}", std::process::id()));
        let file = dir.join("collections.json");
        let mut c = Collections::default();
        c.toggle_like(&p("x"));
        let id = c.create("Mix");
        c.add(id, &[p("y")]);
        c.save(&file).unwrap();
        assert_eq!(Collections::load(&file), c);
        let _ = std::fs::remove_dir_all(dir);
    }
}
