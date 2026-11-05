// Browser-only stand-in for the Rust backend (`pnpm dev` and the end-to-end
// tests). The fake library is large enough to exercise virtualisation and
// includes very long and right-to-left names to catch layout overflow.

import type {
  Collections,
  LibraryStatus,
  PlayerCommand,
  PlayerState,
  QueueSnapshot,
  Settings,
  Track,
} from "./api";

const words = ["Neon", "Glass", "Echo", "Velvet", "Static", "Paper", "Northern", "Quiet", "Golden", "Hollow"];
const nouns = ["Rivers", "Signals", "Afterglow", "Harbor", "Machines", "Gardens", "Lights", "Avenue"];
const artists = [
  "The Placeholders",
  "Lorem Ipsum",
  "Demo Tapes",
  "Null Pointer",
  "گروه نمونه",
  "An Artist With A Remarkably Long Name That Never Seems To End",
];
const params = new URLSearchParams(location.search);
const empty = params.get("library") === "empty";

export const tracks: Track[] = empty
  ? []
  : Array.from({ length: 2400 }, (_, id) => {
      const albumNo = Math.floor(id / 12);
      const artist = artists[albumNo % artists.length];
      const longTitle = id % 37 === 5;
      return {
        id,
        path: `C:/Music/${albumNo}/${id}.flac`,
        title: longTitle
          ? "An Extraordinarily Long Track Title (Extended Remix) [Remastered Deluxe Edition 2026]"
          : id % 29 === 3
            ? "آهنگ نمونه با عنوان فارسی"
            : `${words[id % words.length]} ${nouns[(id * 7) % nouns.length]}`,
        artist,
        album:
          albumNo % 17 === 4
            ? "A Very Long Album Name That Keeps Going Well Past Any Reasonable Width"
            : `${nouns[albumNo % nouns.length]} Vol. ${(albumNo % 9) + 1}`,
        albumArtist: artist,
        trackNo: (id % 12) + 1,
        discNo: null,
        year: 1990 + (albumNo % 35),
        durationMs: 150_000 + ((id * 7919) % 140_000),
        art: `album-${albumNo}`,
      };
    });

export const status: LibraryStatus = { scanning: false, count: tracks.length, root: "C:/Music" };
export const settings: Settings = {
  theme: "system",
  musicDir: "C:/Users/Someone With A Long Name/Music/Lossless Collection/Main Library",
  playerVolume: 0.8,
  noteShortcut: "ScrollLock",
  clipboardPeek: true,
  showTransfers: true,
};

export const collections: Collections = {
  liked: tracks.filter((t) => t.id % 9 === 0).map((t) => t.path),
  playlists: empty
    ? []
    : [
        { id: 1, name: "Morning focus", tracks: tracks.slice(30, 75).map((t) => t.path) },
        {
          id: 2,
          name: "A playlist with an unusually long name for testing",
          tracks: tracks.slice(200, 212).map((t) => t.path),
        },
        { id: 3, name: "شب‌های آرام", tracks: tracks.slice(400, 430).map((t) => t.path) },
      ],
};

let state: PlayerState = {
  track: tracks[5] ?? null,
  playing: false,
  positionMs: 47_000,
  sampledAtMs: Date.now(),
  volume: 0.8,
  shuffle: false,
  repeat: "off",
  queueLen: tracks.length,
  queueRev: 0,
};
let order: number[] = tracks.map((t) => t.id);
let listener: ((s: PlayerState) => void) | null = null;
let colListener: ((c: Collections) => void) | null = null;

export const player = () => state;

export function queue(): QueueSnapshot {
  const i = Math.max(0, order.indexOf(state.track?.id ?? -1));
  return { tracks: order.map((id) => tracks[id]), index: i };
}

export function onPlayer(cb: (s: PlayerState) => void) {
  listener = cb;
  return Promise.resolve(() => (listener = null));
}

export function onCollections(cb: (c: Collections) => void) {
  colListener = cb;
  return Promise.resolve(() => (colListener = null));
}

function update(patch: Partial<PlayerState>) {
  const now = Date.now();
  const pos = state.playing ? state.positionMs + (now - state.sampledAtMs) : state.positionMs;
  state = { ...state, positionMs: pos, sampledAtMs: now, ...patch };
  listener?.(state);
}

function collectionsChanged() {
  colListener?.(structuredClone(collections));
}

function step(delta: number) {
  const i = order.indexOf(state.track?.id ?? -1);
  const next = tracks[order[(i + delta + order.length) % order.length]];
  update({ track: next, positionMs: 0, playing: true });
}

export function play(ids: number[], start: number) {
  order = [...ids];
  update({ track: tracks[ids[start]], positionMs: 0, playing: true, queueLen: ids.length, queueRev: state.queueRev + 1 });
}

export function enqueue(ids: number[], next: boolean) {
  const i = order.indexOf(state.track?.id ?? -1);
  if (next) order.splice(i + 1, 0, ...ids);
  else order.push(...ids);
  update({ queueLen: order.length, queueRev: state.queueRev + 1 });
}

export function command(c: PlayerCommand) {
  switch (c.type) {
    case "togglePlay":
      return update({ playing: !state.playing });
    case "play":
    case "pause":
      return update({ playing: c.type === "play" });
    case "next":
      return step(1);
    case "previous":
      return step(-1);
    case "seek":
      return update({ positionMs: c.positionMs });
    case "setVolume":
      return update({ volume: c.volume });
    case "setShuffle":
      return update({ shuffle: c.on, queueRev: state.queueRev + 1 });
    case "setRepeat":
      return update({ repeat: c.repeat });
    case "jumpTo":
      return update({ track: tracks[order[c.index]], positionMs: 0, playing: true });
    case "removeAt":
      order.splice(c.index, 1);
      return update({ queueLen: order.length, queueRev: state.queueRev + 1 });
  }
}

export function toggleLike(path: string) {
  const i = collections.liked.indexOf(path);
  if (i >= 0) collections.liked.splice(i, 1);
  else collections.liked.unshift(path);
  collectionsChanged();
}

let nextId = 10;
export function playlistCreate(name: string, ids: number[]) {
  const id = nextId++;
  collections.playlists.push({ id, name: name || "New playlist", tracks: ids.map((i) => tracks[i].path) });
  collectionsChanged();
  return id;
}

export function playlistRename(id: number, name: string) {
  const p = collections.playlists.find((p) => p.id === id);
  if (p && name.trim()) p.name = name.trim();
  collectionsChanged();
}

export function playlistDelete(id: number) {
  collections.playlists = collections.playlists.filter((p) => p.id !== id);
  collectionsChanged();
}

export function playlistAdd(id: number, ids: number[]) {
  const p = collections.playlists.find((p) => p.id === id);
  let added = 0;
  for (const i of ids) {
    if (p && !p.tracks.includes(tracks[i].path)) {
      p.tracks.push(tracks[i].path);
      added++;
    }
  }
  collectionsChanged();
  return added;
}

export function playlistRemove(id: number, positions: number[]) {
  const p = collections.playlists.find((p) => p.id === id);
  if (p) p.tracks = p.tracks.filter((_, i) => !positions.includes(i));
  collectionsChanged();
}

export function setOption(key: "clipboardPeek" | "showTransfers", on: boolean) {
  settings[key] = on;
}

export function setShortcut(shortcut: string) {
  if (/^[A-Z0-9]$/i.test(shortcut)) throw new Error(`"${shortcut}" needs Ctrl, Alt, Shift or Win`);
  settings.noteShortcut = shortcut;
  return shortcut;
}
