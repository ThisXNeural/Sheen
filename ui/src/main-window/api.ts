import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { inTauri } from "../shared/env";
import * as demo from "./demo";

// Mirrors sheen_player::{Track, PlayerState, PlayerCommand, Collections} and
// src-tauri/src/{settings,music}.rs. Keep in sync.

export interface Track {
  id: number;
  path: string;
  title: string;
  artist: string;
  album: string;
  albumArtist: string;
  trackNo: number | null;
  discNo: number | null;
  year: number | null;
  durationMs: number;
  art: string;
}

export type Repeat = "off" | "all" | "one";

export interface PlayerState {
  track: Track | null;
  playing: boolean;
  positionMs: number;
  sampledAtMs: number;
  volume: number;
  shuffle: boolean;
  repeat: Repeat;
  queueLen: number;
  queueRev: number;
}

export type PlayerCommand =
  | { type: "togglePlay" }
  | { type: "play" }
  | { type: "pause" }
  | { type: "next" }
  | { type: "previous" }
  | { type: "seek"; positionMs: number }
  | { type: "setVolume"; volume: number }
  | { type: "setShuffle"; on: boolean }
  | { type: "setRepeat"; repeat: Repeat }
  | { type: "jumpTo"; index: number }
  | { type: "removeAt"; index: number };

export interface QueueSnapshot {
  tracks: Track[];
  index: number;
}

export interface LibraryStatus {
  scanning: boolean;
  count: number;
  root: string | null;
}

export interface Playlist {
  id: number;
  name: string;
  tracks: string[];
}

export interface Collections {
  liked: string[];
  playlists: Playlist[];
}

export interface Settings {
  theme: "system" | "light" | "dark";
  musicDir: string | null;
  playerVolume: number;
  noteShortcut: string;
  clipboardPeek: boolean;
  showTransfers: boolean;
}

function on<T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  return inTauri ? listen<T>(event, (e) => cb(e.payload)) : Promise.resolve(() => {});
}

const call = <T>(cmd: string, args: Record<string, unknown> | undefined, fallback: () => T | Promise<T>) =>
  inTauri ? invoke<T>(cmd, args) : Promise.resolve(fallback());

export const onPlayer = (cb: (s: PlayerState) => void) => (inTauri ? on("sheen://player", cb) : demo.onPlayer(cb));
export const onLibrary = (cb: (s: LibraryStatus) => void) => on("sheen://library", cb);
export const onSettings = (cb: (s: Settings) => void) => on("sheen://settings", cb);
export const onCollections = (cb: (c: Collections) => void) =>
  inTauri ? on("sheen://collections", cb) : demo.onCollections(cb);

export const libraryTracks = async (): Promise<Track[]> =>
  inTauri ? (await invoke<{ tracks: Track[] }>("library_tracks")).tracks : demo.tracks;
export const libraryStatus = () => call<LibraryStatus>("library_status", undefined, () => demo.status);
export const getSettings = () => call<Settings>("get_settings", undefined, () => demo.settings);
export const playerState = () => call<PlayerState | null>("player_state", undefined, () => demo.player());
export const queueSnapshot = () => call<QueueSnapshot>("queue_snapshot", undefined, () => demo.queue());
export const collections = () => call<Collections>("collections", undefined, () => demo.collections);

export const rescanLibrary = () => call("rescan_library", undefined, () => undefined);
export const chooseMusicFolder = () => call<string | null>("choose_music_folder", undefined, () => null);
export const playTracks = (ids: number[], start: number) =>
  call("play_tracks", { ids, start }, () => demo.play(ids, start));
export const enqueue = (ids: number[], next: boolean) =>
  call("enqueue_tracks", { ids, next }, () => demo.enqueue(ids, next));
export const command = (command: PlayerCommand) => call("player_command", { command }, () => demo.command(command));
export const saveVolume = (volume: number) => call("save_player_volume", { volume }, () => undefined);
export const reveal = (path: string) => call("reveal_track", { path }, () => undefined);

export const toggleLike = (path: string) => call("toggle_like", { path }, () => demo.toggleLike(path));
export const playlistCreate = (name: string, ids: number[]) =>
  call<number>("playlist_create", { name, ids }, () => demo.playlistCreate(name, ids));
export const playlistRename = (id: number, name: string) =>
  call("playlist_rename", { id, name }, () => demo.playlistRename(id, name));
export const playlistDelete = (id: number) => call("playlist_delete", { id }, () => demo.playlistDelete(id));
export const playlistAdd = (id: number, ids: number[]) =>
  call<number>("playlist_add", { id, ids }, () => demo.playlistAdd(id, ids));
export const playlistRemove = (id: number, positions: number[]) =>
  call("playlist_remove", { id, positions }, () => demo.playlistRemove(id, positions));

export const setIslandOption = (key: "clipboardPeek" | "showTransfers", on: boolean) =>
  call("set_island_option", { key, on }, () => demo.setOption(key, on));
export const setNoteShortcut = (shortcut: string) =>
  call<string>("set_note_shortcut", { shortcut }, () => demo.setShortcut(shortcut));
export const toggleNote = () => call("note_toggle", undefined, () => undefined);
