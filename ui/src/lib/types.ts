// Mirrors `sheen_core::Presentation` (serde camelCase). Keep in sync.

export type Mode = "hidden" | "idle" | "mediaCompact" | "osd" | "expanded";

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
  | { kind: "deviceBattery"; name: string; percent: number };

export interface Presentation {
  mode: Mode;
  geometry: Geometry;
  hovering: boolean;
  media: MediaInfo | null;
  osd: Osd | null;
}

export type MediaCommand =
  | { type: "togglePlayPause" }
  | { type: "next" }
  | { type: "previous" }
  | { type: "seek"; positionMs: number };
