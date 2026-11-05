<script lang="ts">
  import { onMount } from "svelte";
  import Cover from "../shared/Cover.svelte";
  import Icon from "../shared/Icon.svelte";
  import { formatTime, livePosition } from "../shared/format";
  import * as api from "./api";
  import { router } from "./router.svelte";
  import { store } from "./store.svelte";

  let { queueOpen = $bindable(false) }: { queueOpen?: boolean } = $props();

  const player = $derived(store.player);
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
  let beforeMute = 0.8;

  function toggleMute() {
    const v = shownVolume > 0 ? 0 : beforeMute || 0.8;
    if (shownVolume > 0) beforeMute = shownVolume;
    api.command({ type: "setVolume", volume: v });
    api.saveVolume(v);
  }

  const cmd = api.command;
  const nextRepeat = { off: "all", all: "one", one: "off" } as const;
  const liked = $derived(track ? store.liked.has(track.path) : false);
</script>

<footer class:empty={!track}>
  <div class="now">
    <Cover artKey={track?.art} size={52} radius={8} />
    <div class="meta">
      <button
        class="title"
        dir="auto"
        disabled={!track?.album}
        onclick={() => track && router.go({ view: "album", key: track.art })}>{track?.title ?? "Nothing playing"}</button
      >
      <button
        class="artist"
        dir="auto"
        disabled={!track}
        onclick={() => track && router.go({ view: "artist", name: track.albumArtist || track.artist })}
        >{track?.artist ?? ""}</button
      >
    </div>
    {#if track}
      <button
        class="like"
        class:on={liked}
        aria-label={liked ? "Remove from Liked Songs" : "Add to Liked Songs"}
        onclick={() => api.toggleLike(track.path)}><Icon name="heart" size={17} filled={liked} /></button
      >
    {/if}
  </div>

  <div class="center">
    <div class="controls">
      <button
        class="toggle"
        class:on={player?.shuffle}
        aria-label="Shuffle"
        aria-pressed={player?.shuffle ?? false}
        onclick={() => cmd({ type: "setShuffle", on: !player?.shuffle })}><Icon name="shuffle" size={17} /></button
      >
      <button aria-label="Previous" onclick={() => cmd({ type: "previous" })}><Icon name="previous" size={19} filled /></button>
      <button class="play" aria-label={player?.playing ? "Pause" : "Play"} onclick={() => cmd({ type: "togglePlay" })}>
        <Icon name={player?.playing ? "pause" : "play"} size={18} filled />
      </button>
      <button aria-label="Next" onclick={() => cmd({ type: "next" })}><Icon name="next" size={19} filled /></button>
      <button
        class="toggle"
        class:on={player && player.repeat !== "off"}
        aria-label="Repeat: {player?.repeat ?? 'off'}"
        onclick={() => cmd({ type: "setRepeat", repeat: nextRepeat[player?.repeat ?? "off"] })}
      >
        <Icon name={player?.repeat === "one" ? "repeatOne" : "repeat"} size={17} />
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

  <div class="right">
    <button
      class="icon-btn"
      class:on={queueOpen}
      aria-label="Queue"
      aria-pressed={queueOpen}
      onclick={() => (queueOpen = !queueOpen)}><Icon name="queue" size={18} /></button
    >
    <button class="icon-btn" aria-label={shownVolume === 0 ? "Unmute" : "Mute"} onclick={toggleMute}>
      <Icon name={shownVolume === 0 ? "muted" : shownVolume < 0.5 ? "volumeLow" : "volume"} size={18} />
    </button>
    <input
      class="vol"
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
    grid-template-columns: minmax(0, 1fr) minmax(260px, 1.5fr) minmax(0, 1fr);
    align-items: center;
    gap: 16px;
    height: 80px;
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
  .meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .title,
  .artist {
    max-width: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    text-align: left;
  }
  .title {
    font-weight: 600;
  }
  .artist {
    font-size: 12px;
    color: var(--fg-dim);
  }
  .title:not(:disabled):hover,
  .artist:not(:disabled):hover {
    text-decoration: underline;
  }
  .title:disabled,
  .artist:disabled {
    cursor: default;
  }
  .empty .title {
    color: var(--fg-faint);
    font-weight: 500;
  }
  .like {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    color: var(--fg-faint);
  }
  .like.on {
    color: var(--danger);
  }
  .center {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 18px;
  }
  .controls button {
    display: grid;
    place-items: center;
    color: var(--fg);
    opacity: 0.85;
  }
  .controls button:hover {
    opacity: 1;
  }
  .controls .play {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--fg);
    color: var(--bg-elevated);
    opacity: 1;
  }
  .toggle {
    opacity: 0.5 !important;
  }
  .toggle.on {
    color: var(--accent);
    opacity: 1 !important;
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
    min-width: 0;
  }
  .seek span {
    flex: none;
    width: 38px;
  }
  .seek span:first-child {
    text-align: right;
  }
  .right {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 6px;
    min-width: 0;
    color: var(--fg-dim);
  }
  .icon-btn {
    flex: none;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 8px;
  }
  .icon-btn:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
  .icon-btn.on {
    color: var(--accent);
  }
  .vol {
    width: 100px;
    min-width: 50px;
    flex: 0 1 100px;
  }
</style>
