// Browser-only stand-in for the Rust backend (`pnpm dev`), with a fake
// library large enough to exercise the virtualised list.

import type { LibraryStatus, PlayerCommand, PlayerState, Settings, Track } from "./api";

const words = ["Neon", "Glass", "Echo", "Velvet", "Static", "Paper", "Northern", "Quiet", "Golden", "Hollow"];
const nouns = ["Rivers", "Signals", "Afterglow", "Harbor", "Machines", "Gardens", "Lights", "Avenue"];
const artists = ["The Placeholders", "Lorem Ipsum", "Demo Tapes", "Null Pointer", "Sample Rate"];

export const tracks: Track[] = Array.from({ length: 2400 }, (_, id) => ({
  id,
  path: `C:/Music/${id}.flac`,
  title: `${words[id % words.length]} ${nouns[(id * 7) % nouns.length]}`,
  artist: artists[Math.floor(id / 12) % artists.length],
  album: `${nouns[Math.floor(id / 12) % nouns.length]} Vol. ${(Math.floor(id / 12) % 9) + 1}`,
  durationMs: 150_000 + ((id * 7919) % 140_000),
}));

export const status: LibraryStatus = { scanning: false, count: tracks.length, root: "C:/Music" };
export const settings: Settings = { theme: "system", musicDir: "C:/Music", playerVolume: 0.8 };

let state: PlayerState = {
  track: tracks[3],
  playing: false,
  positionMs: 47_000,
  sampledAtMs: Date.now(),
  volume: 0.8,
  shuffle: false,
  repeat: "off",
  queueLen: tracks.length,
};
let queue: number[] = tracks.map((t) => t.id);
let listener: ((s: PlayerState) => void) | null = null;

export const player = () => state;

export function onPlayer(cb: (s: PlayerState) => void) {
  listener = cb;
  return Promise.resolve(() => (listener = null));
}

function update(patch: Partial<PlayerState>) {
  const now = Date.now();
  const pos = state.playing ? state.positionMs + (now - state.sampledAtMs) : state.positionMs;
  state = { ...state, positionMs: pos, sampledAtMs: now, ...patch };
  listener?.(state);
}

function step(delta: number) {
  const i = queue.indexOf(state.track?.id ?? -1);
  const next = tracks[queue[(i + delta + queue.length) % queue.length]];
  update({ track: next, positionMs: 0, playing: true });
}

export function play(ids: number[], start: number) {
  queue = ids;
  update({ track: tracks[ids[start]], positionMs: 0, playing: true, queueLen: ids.length });
  return Promise.resolve();
}

export function command(c: PlayerCommand) {
  switch (c.type) {
    case "togglePlay":
      update({ playing: !state.playing });
      break;
    case "play":
    case "pause":
      update({ playing: c.type === "play" });
      break;
    case "next":
      step(1);
      break;
    case "previous":
      step(-1);
      break;
    case "seek":
      update({ positionMs: c.positionMs });
      break;
    case "setVolume":
      update({ volume: c.volume });
      break;
    case "setShuffle":
      update({ shuffle: c.on });
      break;
    case "setRepeat":
      update({ repeat: c.repeat });
      break;
  }
  return Promise.resolve();
}
