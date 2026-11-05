import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { inTauri } from "../shared/env";
import * as demo from "./demo";

// Mirrors sheen_player::{Track, PlayerState, PlayerCommand} and
// src-tauri/src/{settings,music}.rs. Keep in sync.

export interface Track {
  id: number;
  path: string;
  title: string;
  artist: string;
  album: string;
  durationMs: number;
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
  | { type: "setRepeat"; repeat: Repeat };

export interface LibraryStatus {
  scanning: boolean;
  count: number;
  root: string | null;
}

export interface Settings {
  theme: "system" | "light" | "dark";
  musicDir: string | null;
  playerVolume: number;
}

function on<T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> {
  return inTauri ? listen<T>(event, (e) => cb(e.payload)) : Promise.resolve(() => {});
}

export const onPlayer = (cb: (s: PlayerState) => void) =>
  inTauri ? on("sheen://player", cb) : demo.onPlayer(cb);
export const onLibrary = (cb: (s: LibraryStatus) => void) => on("sheen://library", cb);
export const onSettings = (cb: (s: Settings) => void) => on("sheen://settings", cb);

export const libraryTracks = async (): Promise<Track[]> =>
  inTauri ? (await invoke<{ tracks: Track[] }>("library_tracks")).tracks : demo.tracks;
export const libraryStatus = () =>
  inTauri ? invoke<LibraryStatus>("library_status") : Promise.resolve(demo.status);
export const getSettings = () =>
  inTauri ? invoke<Settings>("get_settings") : Promise.resolve(demo.settings);
export const playerState = () =>
  inTauri ? invoke<PlayerState | null>("player_state") : Promise.resolve(demo.player());
export const playerCover = () =>
  inTauri ? invoke<string | null>("player_cover") : Promise.resolve(null);

export const rescanLibrary = () => (inTauri ? invoke("rescan_library") : Promise.resolve());
export const chooseMusicFolder = () =>
  inTauri ? invoke<string | null>("choose_music_folder") : Promise.resolve(null);
export const playTracks = (ids: number[], start: number) =>
  inTauri ? invoke("play_tracks", { ids, start }) : demo.play(ids, start);
export const command = (command: PlayerCommand) =>
  inTauri ? invoke("player_command", { command }) : demo.command(command);
export const saveVolume = (volume: number) =>
  inTauri ? invoke("save_player_volume", { volume }) : Promise.resolve();
