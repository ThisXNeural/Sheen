//! Glue between the built-in player, the library, the UI and Windows' media
//! controls.

use crate::Shared;
use base64::Engine as _;
use serde::Serialize;
use sheen_player::{library, Library, Player, PlayerCommand, PlayerState, Track};
use sheen_win::{MediaButton, NowPlaying, NowPlayingPublisher, NowPlayingTrack};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager};

pub const EV_PLAYER: &str = "sheen://player";
pub const EV_LIBRARY: &str = "sheen://library";

/// Encoded cover art: `(mime, bytes)`.
type Cover = (String, Vec<u8>);

pub struct Music {
    pub player: Player,
    pub library: Mutex<Arc<Library>>,
    pub state: Mutex<Option<PlayerState>>,
    pub scanning: Mutex<bool>,
    /// Cover of the current track, cached so the UI and SMTC share one read.
    cover: Mutex<Option<(PathBuf, Option<Cover>)>>,
    cache_path: PathBuf,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatus {
    pub scanning: bool,
    pub count: usize,
    pub root: Option<PathBuf>,
}

impl Music {
    /// Start the player thread and register it with SMTC via `hwnd`.
    pub fn start(app: &AppHandle, hwnd: isize, volume: f32) -> Arc<Music> {
        let cache_path = app
            .path()
            .app_cache_dir()
            .unwrap_or_else(|_| std::env::temp_dir().join("sheen"))
            .join("library.json");

        // The player callback needs the `Music` it belongs to; it is created
        // just below, so hand it over through a cell.
        let cell: Arc<OnceLock<Arc<Music>>> = Arc::new(OnceLock::new());
        let publisher: Arc<OnceLock<NowPlayingPublisher>> = Arc::new(OnceLock::new());

        let app2 = app.clone();
        let (cell2, publisher2) = (cell.clone(), publisher.clone());
        let player = Player::spawn(volume, move |state| {
            let Some(music) = cell2.get() else { return };
            *music.state.lock().unwrap() = Some(state.clone());
            let _ = app2.emit(EV_PLAYER, state);
            if let Some(publisher) = publisher2.get() {
                publisher.update(music.now_playing(state));
            }
        });

        let music = Arc::new(Music {
            player: player.clone(),
            library: Mutex::new(Arc::new(Library::load(&cache_path))),
            state: Mutex::new(None),
            scanning: Mutex::new(false),
            cover: Mutex::new(None),
            cache_path,
        });
        let _ = cell.set(music.clone());

        let _ = publisher.set(NowPlayingPublisher::start(hwnd, move |button| {
            player.send(match button {
                MediaButton::Play => PlayerCommand::Play,
                MediaButton::Pause => PlayerCommand::Pause,
                MediaButton::Next => PlayerCommand::Next,
                MediaButton::Previous => PlayerCommand::Previous,
                MediaButton::Seek(ms) => PlayerCommand::Seek { position_ms: ms },
            })
        }));
        music
    }

    pub fn status(&self) -> LibraryStatus {
        let lib = self.library.lock().unwrap();
        LibraryStatus {
            scanning: *self.scanning.lock().unwrap(),
            count: lib.tracks.len(),
            root: lib.root.clone(),
        }
    }

    /// Rescan `root` on a background thread; unchanged files come from cache.
    pub fn rescan(self: &Arc<Self>, app: &AppHandle, root: PathBuf) {
        {
            let mut scanning = self.scanning.lock().unwrap();
            if *scanning {
                return;
            }
            *scanning = true;
        }
        let _ = app.emit(EV_LIBRARY, self.status());

        let (music, app) = (self.clone(), app.clone());
        std::thread::Builder::new()
            .name("sheen-library-scan".into())
            .spawn(move || {
                let started = std::time::Instant::now();
                let previous = music.library.lock().unwrap().clone();
                let fresh = previous.rescan(&root);
                log::info!(
                    "library: {} tracks in {:?}",
                    fresh.tracks.len(),
                    started.elapsed()
                );
                if let Err(e) = fresh.save(&music.cache_path) {
                    log::warn!("could not save library cache: {e}");
                }
                *music.library.lock().unwrap() = Arc::new(fresh);
                *music.scanning.lock().unwrap() = false;
                let _ = app.emit(EV_LIBRARY, music.status());
            })
            .expect("spawn library scan");
    }

    pub fn play_ids(&self, ids: &[u32], start: usize) {
        let lib = self.library.lock().unwrap().clone();
        let tracks: Vec<Track> = ids
            .iter()
            .filter_map(|&id| lib.tracks.get(id as usize).cloned())
            .collect();
        if !tracks.is_empty() {
            self.player.send(PlayerCommand::PlayQueue { tracks, start });
        }
    }

    /// Cover of `path` as `(mime, bytes)`, cached for the current track.
    fn cover(&self, path: &Path) -> Option<Cover> {
        let mut cache = self.cover.lock().unwrap();
        match &*cache {
            Some((p, cover)) if p == path => cover.clone(),
            _ => {
                let cover = library::cover_art(path);
                *cache = Some((path.to_path_buf(), cover.clone()));
                cover
            }
        }
    }

    pub fn current_cover_url(&self) -> Option<String> {
        let path = self
            .state
            .lock()
            .unwrap()
            .as_ref()?
            .track
            .as_ref()?
            .path
            .clone();
        let (mime, bytes) = self.cover(&path)?;
        Some(format!(
            "data:{mime};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ))
    }

    fn now_playing(&self, state: &PlayerState) -> NowPlaying {
        NowPlaying {
            track: state.track.as_ref().map(|t| NowPlayingTrack {
                key: hash(&t.path),
                title: t.title.clone(),
                artist: t.artist.clone(),
                album: t.album.clone(),
                duration_ms: t.duration_ms,
                cover: self.cover(&t.path).map(|(_, bytes)| bytes),
            }),
            playing: state.playing,
            position_ms: state.position_ms,
        }
    }
}

fn hash(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    h.finish()
}

// ---- commands ---------------------------------------------------------------

type Ctx<'a> = tauri::State<'a, Arc<Shared>>;

#[tauri::command]
pub fn library_tracks(state: Ctx) -> Arc<Library> {
    state.music.library.lock().unwrap().clone()
}

#[tauri::command]
pub fn library_status(state: Ctx) -> LibraryStatus {
    state.music.status()
}

#[tauri::command]
pub fn rescan_library(app: AppHandle, state: Ctx) {
    if let Some(root) = state.settings.lock().unwrap().music_dir.clone() {
        state.music.rescan(&app, root);
    }
}

#[tauri::command]
pub async fn choose_music_folder(app: AppHandle) -> Option<PathBuf> {
    use tauri_plugin_dialog::DialogExt;
    let picker = app.clone();
    let folder = tauri::async_runtime::spawn_blocking(move || {
        picker
            .dialog()
            .file()
            .set_title("Choose your music folder")
            .blocking_pick_folder()
    })
    .await
    .ok()??
    .into_path()
    .ok()?;

    let state = app.state::<Arc<Shared>>();
    state.update_settings(&app, |s| s.music_dir = Some(folder.clone()));
    state.music.rescan(&app, folder.clone());
    Some(folder)
}

#[tauri::command]
pub fn play_tracks(state: Ctx, ids: Vec<u32>, start: usize) {
    state.music.play_ids(&ids, start);
}

#[tauri::command]
pub fn player_command(state: Ctx, command: PlayerCommand) {
    state.music.player.send(command);
}

/// Persist the player volume. Called when a volume drag ends, not on every
/// intermediate value, to avoid a disk write per pixel.
#[tauri::command]
pub fn save_player_volume(app: AppHandle, state: Ctx, volume: f32) {
    state.update_settings(&app, |s| s.player_volume = volume.clamp(0.0, 1.0));
}

#[tauri::command]
pub fn player_state(state: Ctx) -> Option<PlayerState> {
    state.music.state.lock().unwrap().clone()
}

#[tauri::command]
pub fn player_cover(state: Ctx) -> Option<String> {
    state.music.current_cover_url()
}
