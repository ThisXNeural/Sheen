<script lang="ts">
  import { onMount } from "svelte";
  import { formatTime, livePosition } from "../shared/format";
  import * as api from "./api";
  import Icon from "./Icon.svelte";

  let { player }: { player: api.PlayerState | null } = $props();

  const track = $derived(player?.track ?? null);
  const duration = $derived(track?.durationMs ?? 0);

  // Interpolate the playhead locally; only tick while actually playing.
  let now = $state(Date.now());
  onMount(() => {
    const id = setInterval(() => {
      if (player?.playing) now = Date.now();
    }, 250);
    return () => clearInterval(id);
  });
  const position = $derived(
    player ? livePosition(player.positionMs, player.sampledAtMs, player.playing, duration || null, now) : 0,
  );

  // While dragging, the slider shows the drag value instead of the playhead.
  let scrubbing = $state<number | null>(null);
  const shown = $derived(scrubbing ?? position);

  let volume = $state<number | null>(null);
  const shownVolume = $derived(volume ?? player?.volume ?? 0.8);

  // Cover art is fetched once per track, not per state update.
  let cover = $state<string | null>(null);
  let coverFor: string | null = null;
  $effect(() => {
    const path = track?.path ?? null;
    if (path === coverFor) return;
    coverFor = path;
    cover = null;
    if (path) api.playerCover().then((url) => coverFor === path && (cover = url));
  });

  const cmd = api.command;
  const nextRepeat = { off: "all", all: "one", one: "off" } as const;
</script>

<footer class:empty={!track}>
  <div class="now">
    {#if cover}
      <img src={cover} alt="" />
    {:else}
      <div class="art"><Icon name="music" size={20} /></div>
    {/if}
    <div class="meta">
      <div class="title">{track?.title ?? "Nothing playing"}</div>
      <div class="artist">{track?.artist ?? ""}</div>
    </div>
  </div>

  <div class="center">
    <div class="controls">
      <button
        class="toggle"
        class:on={player?.shuffle}
        aria-label="Shuffle"
        aria-pressed={player?.shuffle ?? false}
        onclick={() => cmd({ type: "setShuffle", on: !player?.shuffle })}><Icon name="shuffle" size={16} /></button
      >
      <button aria-label="Previous" onclick={() => cmd({ type: "previous" })}><Icon name="previous" /></button>
      <button class="play" aria-label={player?.playing ? "Pause" : "Play"} onclick={() => cmd({ type: "togglePlay" })}>
        <Icon name={player?.playing ? "pause" : "play"} size={20} />
      </button>
      <button aria-label="Next" onclick={() => cmd({ type: "next" })}><Icon name="next" /></button>
      <button
        class="toggle"
        class:on={player && player.repeat !== "off"}
        aria-label="Repeat: {player?.repeat ?? 'off'}"
        onclick={() => cmd({ type: "setRepeat", repeat: nextRepeat[player?.repeat ?? "off"] })}
      >
        <Icon name="repeat" size={16} />
        {#if player?.repeat === "one"}<span class="one">1</span>{/if}
      </button>
    </div>
    <div class="seek">
      <span>{formatTime(shown)}</span>
      <input
        type="range"
        min="0"
        max={duration || 1}
        step="1000"
        value={shown}
        disabled={!track}
        aria-label="Seek"
        style:--fill="{duration ? (shown / duration) * 100 : 0}%"
        oninput={(e) => (scrubbing = +e.currentTarget.value)}
        onchange={(e) => {
          cmd({ type: "seek", positionMs: +e.currentTarget.value });
          scrubbing = null;
        }}
      />
      <span>{formatTime(duration)}</span>
    </div>
  </div>

  <div class="volume">
    <Icon name={shownVolume === 0 ? "muted" : "volume"} size={18} />
    <input
      type="range"
      min="0"
      max="1"
      step="0.01"
      value={shownVolume}
      aria-label="Volume"
      style:--fill="{shownVolume * 100}%"
      oninput={(e) => {
        volume = +e.currentTarget.value;
        cmd({ type: "setVolume", volume });
      }}
      onchange={(e) => {
        api.saveVolume(+e.currentTarget.value);
        volume = null;
      }}
    />
  </div>
</footer>

<style>
  footer {
    display: grid;
    grid-template-columns: 1fr minmax(280px, 1.4fr) 1fr;
    align-items: center;
    gap: 16px;
    height: 76px;
    padding: 0 18px;
    background: var(--bg-elevated);
    border-top: 1px solid var(--border);
  }
  .now {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  img,
  .art {
    width: 48px;
    height: 48px;
    border-radius: 8px;
    object-fit: cover;
    flex: none;
  }
  .art {
    display: grid;
    place-items: center;
    background: var(--bg-active);
    color: var(--fg-faint);
  }
  .meta {
    min-width: 0;
  }
  .title,
  .artist {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .title {
    font-weight: 600;
  }
  .artist {
    color: var(--fg-dim);
  }
  .empty .title {
    color: var(--fg-faint);
    font-weight: 500;
  }
  .center {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 18px;
  }
  .controls button {
    position: relative;
    color: var(--fg);
    opacity: 0.85;
  }
  .controls button:hover {
    opacity: 1;
  }
  .controls .play {
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--fg);
    color: var(--bg-elevated);
    opacity: 1;
  }
  .toggle {
    opacity: 0.45 !important;
  }
  .toggle.on {
    color: var(--accent);
    opacity: 1 !important;
  }
  .one {
    position: absolute;
    right: -6px;
    top: -4px;
    font-size: 9px;
    font-weight: 700;
  }
  .seek {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
  }
  .seek input {
    flex: 1;
  }
  .seek span {
    width: 38px;
  }
  .seek span:first-child {
    text-align: right;
  }
  .volume {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    color: var(--fg-dim);
  }
  .volume input {
    width: 110px;
  }
</style>
