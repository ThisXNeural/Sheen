import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { startDemo } from "./demo";
import type { MediaCommand, Presentation } from "./types";

// Event and command names are defined in src-tauri/src/main.rs.

import { inTauri } from "../shared/env";

export function onPresentation(cb: (p: Presentation) => void): Promise<UnlistenFn> {
  if (!inTauri) return Promise.resolve(startDemo(cb));
  return listen<Presentation>("sheen://presentation", (e) => cb(e.payload));
}

/** Album art as a data: URL, sent only when it changes. */
export function onArtwork(cb: (url: string | null) => void): Promise<UnlistenFn> {
  if (!inTauri) return Promise.resolve(() => {});
  return listen<string | null>("sheen://artwork", (e) => cb(e.payload));
}

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> =>
  inTauri ? invoke<T>(cmd, args) : Promise.resolve(null);

export const currentPresentation = () => call<Presentation>("current_presentation");
export const currentArtwork = () => call<string>("current_artwork");
export const expand = () => call("expand");
export const collapse = () => call("collapse");
export const mediaCommand = (command: MediaCommand) => call("media_command", { command });
export const adjustVolume = (delta: number) => call("adjust_volume", { delta });
export const openMainWindow = () => call("open_main_window");
