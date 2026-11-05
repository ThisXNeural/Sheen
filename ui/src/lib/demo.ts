// Browser-only stand-in for the Rust backend, so the island can be iterated
// on with `pnpm dev` and tested in any browser. It cycles through the modes
// using the same geometry as sheen-core; `?mode=…` pins one.

import type { Geometry, MediaInfo, Presentation, ShelfItem, Transfer } from "./types";

const G = {
  idle: { width: 120, height: 30, radius: 15 },
  idleShelf: { width: 150, height: 30, radius: 15 },
  mediaCompact: { width: 230, height: 36, radius: 18 },
  mediaPeek: { width: 246, height: 40, radius: 20 },
  osd: { width: 300, height: 44, radius: 22 },
  osdTall: { width: 320, height: 64, radius: 26 },
  transfer: { width: 250, height: 36, radius: 18 },
  transferPeek: { width: 340, height: 74, radius: 26 },
  shelf: { width: 420, height: 132, radius: 30 },
  expanded: { width: 400, height: 156, radius: 34 },
} satisfies Record<string, Geometry>;

const long = new URLSearchParams(location.search).has("long");

const media: MediaInfo = {
  sourceApp: "Spotify.exe",
  title: long ? "An Extraordinarily Long Track Title That Will Not Fit (Extended Mix)" : "Neon Afterglow",
  artist: long ? "An Artist With A Remarkably Long Name That Never Ends" : "The Placeholders",
  album: "Demo Mode",
  playing: true,
  positionMs: 61_000,
  durationMs: 243_000,
  sampledAtMs: Date.now(),
};

const download: Transfer = {
  id: 1,
  kind: "download",
  app: "Chrome",
  name: long ? "ubuntu-26.04-desktop-amd64-with-a-really-long-file-name.iso" : "ubuntu-26.04-desktop-amd64.iso",
  progress: 0.42,
  doneBytes: 2_480_000_000,
  totalBytes: 5_900_000_000,
  speedBps: 12_400_000,
  etaSecs: 276,
  state: "active",
};

const copy: Transfer = {
  id: 2,
  kind: "copy",
  app: "File Explorer",
  name: "Copying 1,204 items",
  progress: 0.73,
  doneBytes: null,
  totalBytes: null,
  speedBps: null,
  etaSecs: 48,
  state: "active",
};

const shelf: ShelfItem[] = [
  { id: 1, path: "C:/Users/me/Desktop/Quarterly report.pdf", name: "Quarterly report.pdf", kind: "file", ext: "pdf", size: 2_400_000 },
  { id: 2, path: "C:/Users/me/Pictures/holiday.jpg", name: "holiday.jpg", kind: "image", ext: "jpg", size: 3_100_000 },
  { id: 3, path: "C:/Users/me/Projects", name: "Projects", kind: "folder", ext: "", size: null },
  {
    id: 4,
    path: "C:/Users/me/Downloads/a-file-with-a-very-long-name-indeed.zip",
    name: "a-file-with-a-very-long-name-indeed.zip",
    kind: "file",
    ext: "zip",
    size: 88_000_000,
  },
  { id: 5, path: "C:/notes.txt", name: "notes.txt", kind: "file", ext: "txt", size: 1200 },
];

const base: Presentation = {
  mode: "idle",
  geometry: G.idle,
  hovering: false,
  media: null,
  osd: null,
  transfers: [],
  shelf: [],
  dragging: false,
};

const steps: Record<string, Presentation> = {
  idle: { ...base },
  idleShelf: { ...base, geometry: G.idleShelf, shelf },
  mediaCompact: { ...base, mode: "mediaCompact", geometry: G.mediaCompact, media },
  mediaTransfer: { ...base, mode: "mediaCompact", geometry: G.mediaCompact, media, transfers: [download] },
  osd: { ...base, mode: "osd", geometry: G.osd, media, osd: { kind: "volume", level: 0.62, muted: false } },
  mic: { ...base, mode: "osd", geometry: G.osd, osd: { kind: "micMute", muted: true } },
  clipText: {
    ...base,
    mode: "osd",
    geometry: G.osdTall,
    osd: {
      kind: "clipboard",
      text: long
        ? "A long copied paragraph that goes on and on to make sure the preview clamps to two lines and never spills out of the capsule no matter what"
        : "https://example.com/some/copied/link",
      image: null,
      files: 0,
    },
  },
  clipFiles: {
    ...base,
    mode: "osd",
    geometry: G.osdTall,
    osd: { kind: "clipboard", text: "Quarterly report.pdf", image: null, files: 3 },
  },
  transfer: { ...base, mode: "transfer", geometry: G.transfer, transfers: [download, copy] },
  transferPeek: { ...base, mode: "transfer", geometry: G.transferPeek, hovering: true, transfers: [download, copy] },
  transferDone: { ...base, mode: "transfer", geometry: G.transfer, transfers: [{ ...download, progress: 1, state: "done", etaSecs: null }] },
  shelf: { ...base, mode: "shelf", geometry: G.shelf, hovering: true, shelf },
  shelfDrop: { ...base, mode: "shelf", geometry: G.shelf, dragging: true, shelf: shelf.slice(0, 2) },
  shelfEmpty: { ...base, mode: "shelf", geometry: G.shelf, dragging: true },
  expanded: { ...base, mode: "expanded", geometry: G.expanded, hovering: true, media },
  expandedOsd: {
    ...base,
    mode: "expanded",
    geometry: G.expanded,
    hovering: true,
    media,
    osd: { kind: "volume", level: 0.3, muted: false },
  },
};

export const demoModes = Object.keys(steps);

export function startDemo(onPresentation: (p: Presentation) => void) {
  const pinned = new URLSearchParams(location.search).get("mode");
  if (pinned && steps[pinned]) {
    onPresentation(steps[pinned]);
    return () => {};
  }
  const cycle = ["idle", "mediaCompact", "osd", "transfer", "expanded", "clipText", "shelfDrop", "idleShelf"];
  let i = 0;
  onPresentation(steps[cycle[0]]);
  const id = setInterval(() => onPresentation(steps[cycle[++i % cycle.length]]), 2200);
  return () => clearInterval(id);
}
