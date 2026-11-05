// Browser-only stand-in for the Rust backend, so the UI can be iterated on
// with `pnpm dev` in any browser (no Windows, no Tauri). It cycles through
// the island's modes using the same geometry as sheen-core.

import type { Geometry, MediaInfo, Presentation } from "./types";

const G = {
  idle: { width: 120, height: 30, radius: 15 },
  mediaCompact: { width: 230, height: 36, radius: 18 },
  osd: { width: 300, height: 44, radius: 22 },
  expanded: { width: 400, height: 156, radius: 34 },
} satisfies Record<string, Geometry>;

const media: MediaInfo = {
  sourceApp: "Spotify.exe",
  title: "Neon Afterglow",
  artist: "The Placeholders",
  album: "Demo Mode",
  playing: true,
  positionMs: 61_000,
  durationMs: 243_000,
  sampledAtMs: Date.now(),
};

const steps: Presentation[] = [
  { mode: "idle", geometry: G.idle, hovering: false, media: null, osd: null },
  { mode: "mediaCompact", geometry: G.mediaCompact, hovering: false, media, osd: null },
  {
    mode: "osd",
    geometry: G.osd,
    hovering: false,
    media,
    osd: { kind: "volume", level: 0.62, muted: false },
  },
  { mode: "expanded", geometry: G.expanded, hovering: true, media, osd: null },
  {
    mode: "osd",
    geometry: G.osd,
    hovering: false,
    media,
    osd: { kind: "micMute", muted: true },
  },
];

export function startDemo(onPresentation: (p: Presentation) => void) {
  // ?mode=expanded pins a single state (handy for screenshots).
  const pinned = new URLSearchParams(location.search).get("mode");
  const fixed = steps.find((s) => s.mode === pinned);
  if (fixed) {
    onPresentation(fixed);
    return () => {};
  }
  let i = 0;
  onPresentation(steps[0]);
  const id = setInterval(() => onPresentation(steps[++i % steps.length]), 2200);
  return () => clearInterval(id);
}
