//! Glue between the built-in player, the library, the UI and Windows' media
//! controls.

use crate::Shared;
use serde::{Deserialize, Serialize};
use sheen_player::art::ArtCache;
use sheen_player::{
    library, Collections, Library, Player, PlayerCommand, PlayerState, QueueSnapshot, Repeat, Track,
};
use sheen_win::{MediaButton, NowPlaying, NowPlayingPublisher, NowPlayingTrack};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub const EV_PLAYER: &str = "sheen://player";
pub const EV_LIBRARY: &str = "sheen://library";
pub const EV_COLLECTIONS: &str = "sheen://collections";

/// Thumbnail sizes served by the art protocol.
pub const THUMB_PX: u32 = 96;
pub const COVER_PX: u32 = 480;

pub struct Music {
    pub player: Player,
    pub library: Mutex<Arc<Library>>,
    pub state: Mutex<Option<PlayerState>>,
    pub scanning: Mutex<bool>,
    pub collections: Mutex<Collections>,
    pub art: ArtCache,
    /// Art key → a file of that album, to extract the cover from.
    art_sources: Mutex<HashMap<String, PathBuf>>,
    paths: Paths,
    save_session: Sender<()>,
}

struct Paths {
    library: PathBuf,
    collections: PathBuf,
    session: PathBuf,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatus {
    pub scanning: bool,
    pub count: usize,
    pub root: Option<PathBuf>,
}

/// Saved between runs: the queue, where we were, and the play modes.
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Session {
    tracks: Vec<PathBuf>,
    index: usize,
    position_ms: u64,
    shuffle: bool,
    repeat: Repeat,
}

impl Music {
    /// Start the player thread and register it with SMTC via `hwnd`.
    pub fn start(app: &AppHandle, hwnd: isize, volume: f32) -> Arc<Music> {
        let cache =
            app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir().join("sheen"));
        let data = app.path().app_data_dir().unwrap_or_else(|_| cache.clone());
        let paths = Paths {
            library: cache.join("library.json"),
            collections: data.join("collections.json"),
            session: data.join("session.json"),
        };

        // The player callback needs the `Music` it belongs to; it is created
        // just below, so hand it over through a cell.
        let cell: Arc<OnceLock<Arc<Music>>> = Arc::new(OnceLock::new());
        let publisher: Arc<OnceLock<NowPlayingPublisher>> = Arc::new(OnceLock::new());

        let app2 = app.clone();
        let (cell2, publisher2) = (cell.clone(), publisher.clone());
        let player = Player::spawn(volume, move |state| {
            let Some(music) = cell2.get() else { return };
            let previous = music.state.lock().unwrap().replace(state.clone());
            let _ = app2.emit(EV_PLAYER, state);
            if let Some(publisher) = publisher2.get() {
                publisher.update(music.now_playing(state));
            }
            // Persist the session when something worth resuming changed.
            let worth_saving = previous.as_ref().is_none_or(|p| {
                p.queue_rev != state.queue_rev
                    || p.playing != state.playing
                    || p.track.as_ref().map(|t| &t.path) != state.track.as_ref().map(|t| &t.path)
                    || p.shuffle != state.shuffle
                    || p.repeat != state.repeat
            });
            if worth_saving {
                let _ = music.save_session.send(());
            }
        });

        let library = Library::load(&paths.library);
        let collections = Collections::load(&paths.collections);
        let (save_tx, save_rx) = channel::<()>();
        let music = Arc::new(Music {
            player: player.clone(),
            library: Mutex::new(Arc::new(library)),
            state: Mutex::new(None),
            scanning: Mutex::new(false),
            collections: Mutex::new(collections),
            art: ArtCache::new(cache.join("art")),
            art_sources: Mutex::new(HashMap::new()),
            paths,
            save_session: save_tx,
        });
        music.index_art();
        let _ = cell.set(music.clone());

        // Session saver: debounced, and off the player thread (it needs to
        // ask the player for the queue).
        let saver = music.clone();
        std::thread::Builder::new()
            .name("sheen-session".into())
            .spawn(move || {
                while save_rx.recv().is_ok() {
                    std::thread::sleep(Duration::from_millis(1500));
                    while save_rx.try_recv().is_ok() {}
                    saver.write_session();
                }
            })
            .expect("spawn session saver");

        music.restore_session();

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

    fn index_art(&self) {
        let lib = self.library.lock().unwrap().clone();
        let mut map = HashMap::with_capacity(lib.tracks.len() / 8);
        for t in &lib.tracks {
            map.entry(t.art.clone()).or_insert_with(|| t.path.clone());
        }
        *self.art_sources.lock().unwrap() = map;
    }

    /// JPEG cover for an art key, at `size` px.
    pub fn cover(&self, key: &str, size: u32) -> Option<Vec<u8>> {
        let source = self.art_sources.lock().unwrap().get(key).cloned()?;
        self.art.get(key, size, &source)
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
                log::info!("library: {} tracks in {:?}", fresh.tracks.len(), started.elapsed());
                if let Err(e) = fresh.save(&music.paths.library) {
                    log::warn!("could not save library cache: {e}");
                }
                *music.library.lock().unwrap() = Arc::new(fresh);
                music.index_art();
                music.art.reset_misses();
                *music.scanning.lock().unwrap() = false;
                let _ = app.emit(EV_LIBRARY, music.status());
            })
            .expect("spawn library scan");
    }

    fn tracks_for(&self, ids: &[u32]) -> Vec<Track> {
        let lib = self.library.lock().unwrap().clone();
        ids.iter().filter_map(|&id| lib.tracks.get(id as usize).cloned()).collect()
    }

    fn tracks_for_paths(&self, paths: &[PathBuf]) -> Vec<Track> {
        let lib = self.library.lock().unwrap().clone();
        let by_path: HashMap<&Path, &Track> =
            lib.tracks.iter().map(|t| (t.path.as_path(), t)).collect();
        paths.iter().filter_map(|p| by_path.get(p.as_path()).map(|t| (*t).clone())).collect()
    }

    fn write_session(&self) {
        let QueueSnapshot { tracks, index } = self.player.queue();
        let (position_ms, shuffle, repeat) = self
            .state
            .lock()
            .unwrap()
            .as_ref()
            .map_or((0, false, Repeat::Off), |s| (s.position_ms, s.shuffle, s.repeat));
        let session = Session {
            tracks: tracks.into_iter().map(|t| t.path).collect(),
            index,
            position_ms,
            shuffle,
            repeat,
        };
        if let Ok(bytes) = serde_json::to_vec(&session) {
            if let Some(dir) = self.paths.session.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&self.paths.session, bytes);
        }
    }

    /// Final save on exit, including the current playhead.
    pub fn flush_session(&self) {
        self.write_session();
    }

    fn restore_session(&self) {
        let Some(session) = std::fs::read(&self.paths.session)
            .ok()
            .and_then(|b| serde_json::from_slice::<Session>(&b).ok())
        else {
            return;
        };
        let current = session.tracks.get(session.index).cloned();
        let tracks = self.tracks_for_paths(&session.tracks);
        if tracks.is_empty() {
            return;
        }
        // Files may have vanished; find the saved track in what is left.
        let index = current.and_then(|c| tracks.iter().position(|t| t.path == c)).unwrap_or(0);
        self.player.send(PlayerCommand::Restore {
            tracks,
            index,
            position_ms: session.position_ms,
            shuffle: session.shuffle,
            repeat: session.repeat,
        });
    }

    fn update_collections(&self, app: &AppHandle, f: impl FnOnce(&mut Collections)) {
        let snapshot = {
            let mut c = self.collections.lock().unwrap();
            f(&mut c);
            if let Err(e) = c.save(&self.paths.collections) {
                log::warn!("could not save playlists: {e}");
            }
            c.clone()
        };
        let _ = app.emit(EV_COLLECTIONS, snapshot);
    }

    fn now_playing(&self, state: &PlayerState) -> NowPlaying {
        NowPlaying {
            track: state.track.as_ref().map(|t| NowPlayingTrack {
                key: hash(&t.path),
                title: t.title.clone(),
                artist: t.artist.clone(),
                album: t.album.clone(),
                duration_ms: t.duration_ms,
                cover: self
                    .cover(&t.art, COVER_PX)
                    .or_else(|| library::cover_art(&t.path).map(|(_, bytes)| bytes)),
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
        picker.dialog().file().set_title("Choose your music folder").blocking_pick_folder()
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
    let tracks = state.music.tracks_for(&ids);
    if !tracks.is_empty() {
        state.music.player.send(PlayerCommand::PlayQueue { tracks, start });
    }
}

#[tauri::command]
pub fn enqueue_tracks(state: Ctx, ids: Vec<u32>, next: bool) {
    let tracks = state.music.tracks_for(&ids);
    if !tracks.is_empty() {
        state.music.player.send(PlayerCommand::Enqueue { tracks, next });
    }
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
pub fn queue_snapshot(state: Ctx) -> QueueSnapshot {
    state.music.player.queue()
}

#[tauri::command]
pub fn reveal_track(path: PathBuf) {
    sheen_win::reveal_in_explorer(&path);
}

#[tauri::command]
pub fn collections(state: Ctx) -> Collections {
    state.music.collections.lock().unwrap().clone()
}

#[tauri::command]
pub fn toggle_like(app: AppHandle, state: Ctx, path: PathBuf) {
    state.music.update_collections(&app, |c| {
        c.toggle_like(&path);
    });
}

#[tauri::command]
pub fn playlist_create(app: AppHandle, state: Ctx, name: String, ids: Vec<u32>) -> u64 {
    let paths: Vec<PathBuf> = state.music.tracks_for(&ids).into_iter().map(|t| t.path).collect();
    let mut id = 0;
    state.music.update_collections(&app, |c| {
        id = c.create(&name);
        c.add(id, &paths);
    });
    id
}

#[tauri::command]
pub fn playlist_rename(app: AppHandle, state: Ctx, id: u64, name: String) {
    state.music.update_collections(&app, |c| {
        c.rename(id, &name);
    });
}

#[tauri::command]
pub fn playlist_delete(app: AppHandle, state: Ctx, id: u64) {
    state.music.update_collections(&app, |c| {
        c.delete(id);
    });
}

#[tauri::command]
pub fn playlist_add(app: AppHandle, state: Ctx, id: u64, ids: Vec<u32>) -> usize {
    let paths: Vec<PathBuf> = state.music.tracks_for(&ids).into_iter().map(|t| t.path).collect();
    let mut added = 0;
    state.music.update_collections(&app, |c| added = c.add(id, &paths));
    added
}

#[tauri::command]
pub fn playlist_remove(app: AppHandle, state: Ctx, id: u64, positions: Vec<usize>) {
    state.music.update_collections(&app, |c| c.remove(id, &positions));
}

#[tauri::command]
pub fn playlist_reorder(app: AppHandle, state: Ctx, id: u64, from: usize, to: usize) {
    state.music.update_collections(&app, |c| c.reorder(id, from, to));
}
