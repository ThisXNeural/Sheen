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
