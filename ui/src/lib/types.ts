// Mirrors `sheen_core::Presentation` (serde camelCase). Keep in sync.

export type Mode = "hidden" | "idle" | "mediaCompact" | "transfer" | "osd" | "expanded" | "shelf";

export interface Geometry {
  width: number;
  height: number;
  radius: number;
}

export interface MediaInfo {
  sourceApp: string;
  title: string;
  artist: string;
  album: string;
  playing: boolean;
  positionMs: number | null;
  durationMs: number | null;
  sampledAtMs: number;
}

export type Osd =
  | { kind: "volume"; level: number; muted: boolean }
  | { kind: "brightness"; level: number }
  | { kind: "micMute"; muted: boolean }
  | { kind: "capsLock"; on: boolean }
  | { kind: "deviceBattery"; name: string; percent: number }
  | { kind: "clipboard"; text: string | null; image: string | null; files: number };

export interface Transfer {
  id: number;
  kind: "download" | "copy";
  app: string;
  name: string;
  progress: number | null;
  doneBytes: number | null;
  totalBytes: number | null;
  speedBps: number | null;
  etaSecs: number | null;
  state: "active" | "stalled" | "done";
}

export interface ShelfItem {
  id: number;
  path: string;
  name: string;
  kind: "image" | "folder" | "file";
  ext: string;
  size: number | null;
}

export interface Presentation {
  mode: Mode;
  geometry: Geometry;
  hovering: boolean;
  media: MediaInfo | null;
  osd: Osd | null;
  transfers: Transfer[];
  shelf: ShelfItem[];
  dragging: boolean;
  shelfBadge: boolean;
  scale: number;
}

export type MediaCommand =
  | { type: "togglePlayPause" }
  | { type: "next" }
  | { type: "previous" }
  | { type: "seek"; positionMs: number };
