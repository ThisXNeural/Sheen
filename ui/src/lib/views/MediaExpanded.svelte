<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../../shared/Icon.svelte";
  import * as ipc from "../ipc";
  import type { MediaInfo, Osd } from "../types";
  import OsdView from "./Osd.svelte";

  let { media, artwork, osd }: { media: MediaInfo; artwork: string | null; osd: Osd | null } =
    $props();

  // Interpolate the playhead locally instead of streaming it over IPC.
  let now = $state(Date.now());
  onMount(() => {
    const id = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(id);
  });

  const position = $derived.by(() => {
    if (media.positionMs == null) return null;
    const elapsed = media.playing ? now - media.sampledAtMs : 0;
    const pos = media.positionMs + Math.max(0, elapsed);
    return media.durationMs ? Math.min(pos, media.durationMs) : pos;
  });
  const progress = $derived(
    position != null && media.durationMs ? position / media.durationMs : 0,
  );

  function fmt(ms: number | null) {
    if (ms == null) return "--:--";
    const s = Math.floor(ms / 1000);
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  }

  function seek(e: MouseEvent) {
    if (!media.durationMs) return;
    const bar = e.currentTarget as HTMLElement;
    const ratio = (e.clientX - bar.getBoundingClientRect().left) / bar.clientWidth;
    ipc.mediaCommand({ type: "seek", positionMs: Math.round(ratio * media.durationMs) });
  }
</script>

<div class="card">
  {#if artwork}
    <!-- Blurred artwork wash behind the card. -->
    <div class="wash" style:background-image="url({artwork})"></div>
  {/if}

  <div class="top">
    {#if artwork}
      <img src={artwork} alt="" />
    {:else}
      <div class="art placeholder"></div>
    {/if}
    <div class="meta">
      <div class="title" dir="auto">{media.title}</div>
      <div class="artist" dir="auto">{media.artist}</div>
    </div>
  </div>

  {#if osd}
    <div class="osd"><OsdView {osd} inline /></div>
  {:else}
    <div class="timeline">
      <span>{fmt(position)}</span>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="bar" onclick={seek}><div class="fill" style:width="{progress * 100}%"></div></div>
      <span>{fmt(media.durationMs)}</span>
    </div>
  {/if}

  <div class="controls">
    <button aria-label="Previous" onclick={() => ipc.mediaCommand({ type: "previous" })}>
      <Icon name="previous" size={20} filled />
    </button>
    <button
      class="play"
      aria-label={media.playing ? "Pause" : "Play"}
      onclick={() => ipc.mediaCommand({ type: "togglePlayPause" })}
    >
      <Icon name={media.playing ? "pause" : "play"} size={24} filled />
    </button>
    <button aria-label="Next" onclick={() => ipc.mediaCommand({ type: "next" })}>
      <Icon name="next" size={20} filled />
    </button>
  </div>
</div>

<style>
  .card {
    position: relative;
    height: 100%;
    padding: 18px 22px 14px;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .wash {
    position: absolute;
    inset: -40px;
    background-size: cover;
    background-position: center;
    filter: blur(40px) saturate(1.4);
    opacity: 0.45;
    z-index: 0;
  }
  .card > :not(.wash) {
    position: relative;
    z-index: 1;
  }
  .top {
    display: flex;
    gap: 14px;
    align-items: center;
  }
  img,
  .art {
    width: 56px;
    height: 56px;
    border-radius: 12px;
    object-fit: cover;
    flex: none;
  }
  .placeholder {
    background: var(--island-track);
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
    font-size: 15px;
    font-weight: 600;
  }
  .artist {
    color: var(--island-fg-dim);
  }
  .timeline,
  .osd {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 16px;
    font-size: 11px;
    color: var(--island-fg-dim);
    font-variant-numeric: tabular-nums;
  }
  .osd {
    color: var(--island-fg);
  }
  .osd > :global(*) {
    flex: 1;
  }
  .bar {
    flex: 1;
    height: 5px;
    border-radius: 3px;
    background: var(--island-track);
    cursor: pointer;
    overflow: hidden;
  }
  .bar .fill {
    height: 100%;
    background: var(--island-accent);
  }
  .controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 30px;
  }
  .controls button {
    display: grid;
    place-items: center;
  }
</style>
