export function formatTime(ms: number | null | undefined): string {
  if (ms == null || !isFinite(ms)) return "--:--";
  const s = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

/** Playhead extrapolated from the last sample while playing. */
export function livePosition(
  positionMs: number,
  sampledAtMs: number,
  playing: boolean,
  durationMs: number | null,
  now: number,
): number {
  const pos = positionMs + (playing ? Math.max(0, now - sampledAtMs) : 0);
  return durationMs ? Math.min(pos, durationMs) : pos;
}

const UNITS = ["B", "KB", "MB", "GB", "TB"];

export function formatBytes(n: number | null | undefined): string {
  if (n == null || !isFinite(n)) return "";
  let v = n;
  let u = 0;
  while (v >= 1024 && u < UNITS.length - 1) {
    v /= 1024;
    u++;
  }
  return `${v >= 100 || u === 0 ? Math.round(v) : v.toFixed(1)} ${UNITS[u]}`;
}

/** A human remaining-time string that does not flicker between units. */
export function formatEta(secs: number | null | undefined): string {
  if (secs == null) return "";
  if (secs < 5) return "a few seconds left";
  if (secs < 60) return `${Math.ceil(secs / 5) * 5} s left`;
  const m = Math.round(secs / 60);
  if (m < 60) return `${m} min left`;
  const h = Math.floor(m / 60);
  return `${h} h ${m % 60} min left`;
}
