use lofty::config::ParseOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "aac", "ogg", "oga", "wav"];
/// Bump when `Track` gains fields that require re-reading tags.
pub const CACHE_VERSION: u32 = 2;
/// Image files next to the music that count as album art, in priority order.
const FOLDER_ART: &[&str] = &["cover", "folder", "front", "album", "albumart"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    /// Index in the library. Stable until the next rescan.
    pub id: u32,
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Album artist if tagged, otherwise the track artist.
    #[serde(default)]
    pub album_artist: String,
    #[serde(default)]
    pub track_no: Option<u32>,
    #[serde(default)]
    pub disc_no: Option<u32>,
    #[serde(default)]
    pub year: Option<u32>,
    pub duration_ms: u64,
    /// Key shared by every track of an album; used to cache one cover per album.
    #[serde(default)]
    pub art: String,
    /// File modification time (unix seconds), used to skip re-parsing.
    #[serde(default)]
    pub mtime: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Library {
    #[serde(default)]
    pub version: u32,
    pub root: Option<PathBuf>,
    pub tracks: Vec<Track>,
}

impl Library {
    /// Load a previously saved library. Missing or corrupt caches yield an
    /// empty library rather than an error.
    pub fn load(cache: &Path) -> Self {
        let mut lib: Library = std::fs::read(cache)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        if lib.version != CACHE_VERSION {
            // Older caches lack fields; force a full re-read on the next scan.
            lib.tracks.clear();
            lib.version = CACHE_VERSION;
        }
        lib
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

        tracks.sort_by_cached_key(|t| {
            (
                t.album_artist.to_lowercase(),
                t.album.to_lowercase(),
                t.disc_no.unwrap_or(0),
                t.track_no.unwrap_or(u32::MAX),
                t.title.to_lowercase(),
            )
        });
        for (i, t) in tracks.iter_mut().enumerate() {
            t.id = i as u32;
        }
        Library { version: CACHE_VERSION, root: Some(root.to_path_buf()), tracks }
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
        tag.and_then(f).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    };
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let artist = get(|t| t.artist()).unwrap_or_default();
    let album = get(|t| t.album()).unwrap_or_default();
    let album_artist = tag
        .and_then(|t| t.get_string(ItemKey::AlbumArtist))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| artist.clone());

    Some(Track {
        id: 0,
        path: path.to_path_buf(),
        title: get(|t| t.title()).unwrap_or(stem),
        art: art_key(&album_artist, &album, path),
        artist,
        album,
        album_artist,
        track_no: tag.and_then(|t| t.track()),
        disc_no: tag.and_then(|t| t.disk()),
        year: tag.and_then(|t| t.date()).map(|d| d.year as u32).filter(|y| *y > 0),
        duration_ms: file.properties().duration().as_millis() as u64,
        mtime,
    })
}

/// One key per album (or per file when untagged), stable across rescans.
pub fn art_key(album_artist: &str, album: &str, path: &Path) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    if album.is_empty() {
        path.hash(&mut h);
    } else {
        album_artist.to_lowercase().hash(&mut h);
        album.to_lowercase().hash(&mut h);
    }
    format!("{:016x}", h.finish())
}

/// Cover art of a file as `(mime, bytes)`, read on demand: the embedded
/// picture if there is one, otherwise a `cover.jpg`/`folder.png`/... next to it.
pub fn cover_art(path: &Path) -> Option<(String, Vec<u8>)> {
    embedded_art(path).or_else(|| folder_art(path))
}

fn embedded_art(path: &Path) -> Option<(String, Vec<u8>)> {
    let file = lofty::read_from_path(path).ok()?;
    let tag = file.primary_tag().or_else(|| file.first_tag())?;
    let pic = tag.pictures().first()?;
    let mime = pic.mime_type().map_or("image/jpeg", |m| m.as_str()).to_string();
    Some((mime, pic.data().to_vec()))
}

fn folder_art(path: &Path) -> Option<(String, Vec<u8>)> {
    let dir = path.parent()?;
    let mut best: Option<(usize, PathBuf, &str)> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        let (Some(stem), Some(ext)) = (p.file_stem(), p.extension()) else { continue };
        let mime = match ext.to_string_lossy().to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            _ => continue,
        };
        let stem = stem.to_string_lossy().to_ascii_lowercase();
        if let Some(rank) = FOLDER_ART.iter().position(|n| *n == stem) {
            if best.as_ref().is_none_or(|(r, _, _)| rank < *r) {
                best = Some((rank, p, mime));
            }
        }
    }
    let (_, p, mime) = best?;
    Some((mime.to_string(), std::fs::read(p).ok()?))
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
                album_artist: "B".into(),
                track_no: Some(3),
                disc_no: None,
                year: Some(2020),
                duration_ms: 1000,
                art: art_key("B", "C", Path::new("a.mp3")),
                mtime: 42,
            }],
            version: CACHE_VERSION,
        };
        lib.save(&cache).unwrap();
        assert_eq!(Library::load(&cache).tracks, lib.tracks);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn outdated_cache_is_discarded() {
        let dir = std::env::temp_dir().join(format!("sheen-old-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cache = dir.join("library.json");
        std::fs::write(&cache, r#"{"root":"/m","tracks":[{"id":0,"path":"/m/a.mp3","title":"A","artist":"","album":"","durationMs":1}]}"#).unwrap();
        let lib = Library::load(&cache);
        assert!(lib.tracks.is_empty());
        assert_eq!(lib.root.as_deref(), Some(Path::new("/m")));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn album_tracks_share_an_art_key() {
        let a = art_key("Band", "Album", Path::new("/m/1.mp3"));
        let b = art_key("band", "ALBUM", Path::new("/m/2.mp3"));
        assert_eq!(a, b);
        assert_ne!(art_key("", "", Path::new("/m/1.mp3")), art_key("", "", Path::new("/m/2.mp3")));
    }

    #[test]
    fn finds_folder_art_by_priority() {
        let dir = std::env::temp_dir().join(format!("sheen-art-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Folder.JPG"), b"folder").unwrap();
        std::fs::write(dir.join("cover.png"), b"cover").unwrap();
        std::fs::write(dir.join("random.jpg"), b"x").unwrap();
        let (mime, bytes) = folder_art(&dir.join("song.mp3")).unwrap();
        assert_eq!((mime.as_str(), bytes.as_slice()), ("image/png", &b"cover"[..]));
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
