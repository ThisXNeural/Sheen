use lofty::config::ParseOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "aac", "ogg", "oga", "wav"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    /// Index in the library. Stable until the next rescan.
    pub id: u32,
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    /// File modification time (unix seconds), used to skip re-parsing.
    #[serde(default)]
    pub mtime: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Library {
    pub root: Option<PathBuf>,
    pub tracks: Vec<Track>,
}

impl Library {
    /// Load a previously saved library. Missing or corrupt caches yield an
    /// empty library rather than an error.
    pub fn load(cache: &Path) -> Self {
        std::fs::read(cache)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, cache: &Path) -> std::io::Result<()> {
        if let Some(dir) = cache.parent() {
            std::fs::create_dir_all(dir)?;
        }
        // Write-then-rename so a crash never leaves a half-written cache.
        let tmp = cache.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec(self)?)?;
        std::fs::rename(tmp, cache)
    }

    /// Scan `root`, reusing entries from `self` whose file has not changed.
    /// Cover art is not read here; see [`cover_art`].
    pub fn rescan(&self, root: &Path) -> Library {
        let previous: HashMap<&Path, &Track> = if self.root.as_deref() == Some(root) {
            self.tracks.iter().map(|t| (t.path.as_path(), t)).collect()
        } else {
            HashMap::new()
        };

        let mut tracks: Vec<Track> = walkdir::WalkDir::new(root)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file() && is_audio(e.path()))
            .filter_map(|entry| {
                let path = entry.path();
                let mtime = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs());
                match previous.get(path) {
                    Some(t) if t.mtime == mtime => Some((*t).clone()),
                    _ => read_track(path, mtime),
                }
            })
            .collect();

        tracks.sort_by(|a, b| {
            (a.artist.to_lowercase(), a.album.to_lowercase(), &a.path).cmp(&(
                b.artist.to_lowercase(),
                b.album.to_lowercase(),
                &b.path,
            ))
        });
        for (i, t) in tracks.iter_mut().enumerate() {
            t.id = i as u32;
        }
        Library {
            root: Some(root.to_path_buf()),
            tracks,
        }
    }
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

fn read_track(path: &Path, mtime: u64) -> Option<Track> {
    let file = Probe::open(path)
        .ok()?
        .options(ParseOptions::new().read_cover_art(false))
        .read()
        .map_err(|e| log::debug!("skipping {}: {e}", path.display()))
        .ok()?;
    let tag = file.primary_tag().or_else(|| file.first_tag());
    let get = |f: fn(&lofty::tag::Tag) -> Option<std::borrow::Cow<'_, str>>| {
        tag.and_then(f)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    Some(Track {
        id: 0,
        path: path.to_path_buf(),
        title: get(|t| t.title()).unwrap_or(stem),
        artist: get(|t| t.artist()).unwrap_or_default(),
        album: get(|t| t.album()).unwrap_or_default(),
        duration_ms: file.properties().duration().as_millis() as u64,
        mtime,
    })
}

/// Embedded cover art of a single file as `(mime, bytes)`, read on demand.
pub fn cover_art(path: &Path) -> Option<(String, Vec<u8>)> {
    let file = lofty::read_from_path(path).ok()?;
    let tag = file.primary_tag().or_else(|| file.first_tag())?;
    let pic = tag.pictures().first()?;
    let mime = pic
        .mime_type()
        .map_or("image/jpeg", |m| m.as_str())
        .to_string();
    Some((mime, pic.data().to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_audio_extensions() {
        assert!(is_audio(Path::new("a/b/song.MP3")));
        assert!(is_audio(Path::new("x.flac")));
        assert!(!is_audio(Path::new("cover.jpg")));
        assert!(!is_audio(Path::new("noext")));
    }

    #[test]
    fn missing_cache_is_empty_library() {
        let lib = Library::load(Path::new("/definitely/not/here.json"));
        assert!(lib.tracks.is_empty());
    }

    #[test]
    fn cache_roundtrip() {
        let dir = std::env::temp_dir().join(format!("sheen-lib-{}", std::process::id()));
        let cache = dir.join("library.json");
        let lib = Library {
            root: Some(dir.clone()),
            tracks: vec![Track {
                id: 0,
                path: dir.join("a.mp3"),
                title: "A".into(),
                artist: "B".into(),
                album: "C".into(),
                duration_ms: 1000,
                mtime: 42,
            }],
        };
        lib.save(&cache).unwrap();
        assert_eq!(Library::load(&cache).tracks, lib.tracks);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn unreadable_files_are_skipped_on_scan() {
        let dir = std::env::temp_dir().join(format!("sheen-scan-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/broken.mp3"), b"not audio").unwrap();
        std::fs::write(dir.join("notes.txt"), b"hi").unwrap();
        let lib = Library::default().rescan(&dir);
        assert!(lib.tracks.is_empty());
        assert_eq!(lib.root.as_deref(), Some(dir.as_path()));
        let _ = std::fs::remove_dir_all(dir);
    }
}
