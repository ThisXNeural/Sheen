<script lang="ts">
  // Playing, collapsed: only the artwork and the level bars, with the song's
  // progress as a hairline along the bottom. Hovering ("peek") adds the
  // title, artist and times; clicking opens the full card.
  import { onMount } from "svelte";
  import Icon from "../../shared/Icon.svelte";
  import { formatTime, livePosition } from "../../shared/format";
  import type { MediaInfo, Transfer } from "../types";
  import Waveform from "./Waveform.svelte";

  let {
    media,
    artwork,
    transfer = null,
    peek = false,
  }: { media: MediaInfo; artwork: string | null; transfer?: Transfer | null; peek?: boolean } = $props();

  let now = $state(Date.now());
  onMount(() => {
    const id = setInterval(() => media.playing && (now = Date.now()), 500);
    return () => clearInterval(id);
  });
  const position = $derived(
    media.positionMs == null
      ? null
      : livePosition(media.positionMs, media.sampledAtMs, media.playing, media.durationMs, now),
  );
  const progress = $derived(position != null && media.durationMs ? position / media.durationMs : null);
  const pct = $derived(transfer?.progress != null ? `${Math.floor(transfer.progress * 100)}%` : "");
</script>

{#snippet art(size: number)}
  {#if artwork}
    <img src={artwork} alt="" style:width="{size}px" style:height="{size}px" />
  {:else}
    <span class="placeholder" style:width="{size}px" style:height="{size}px"><Icon name="music" size={size * 0.5} /></span>
  {/if}
{/snippet}

{#if peek}
  <div class="peek">
    {@render art(42)}
    <div class="meta">
      <span class="title" dir="auto">{media.title}</span>
      <span class="artist" dir="auto">{media.artist}</span>
      {#if progress != null}
        <div class="bar"><i style:width="{progress * 100}%"></i></div>
      {/if}
    </div>
    <div class="right">
      <Waveform playing={media.playing} />
      {#if position != null}<span class="time">{formatTime(position)}</span>{/if}
    </div>
  </div>
{:else}
  <div class="compact">
    {@render art(24)}
    {#if transfer}
      <span class="transfer" class:done={transfer.state === "done"}>
        <Icon name={transfer.state === "done" ? "check" : "download"} size={12} />
        {transfer.state === "done" ? "Done" : pct}
      </span>
    {/if}
    <Waveform playing={media.playing} />
    {#if progress != null}
      <div class="line"><i style:width="{progress * 100}%"></i></div>
    {/if}
  </div>
{/if}

<style>
  img,
  .placeholder {
    flex: none;
    border-radius: 7px;
    object-fit: cover;
  }
  .placeholder {
    display: grid;
    place-items: center;
    background: var(--island-track);
    color: var(--island-fg-dim);
  }
  .compact {
    position: relative;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 0 14px 0 6px;
    box-sizing: border-box;
  }
  .transfer {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    font-size: 11px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--island-fg-dim);
    white-space: nowrap;
  }
  .transfer.done {
    color: #30d158;
  }
  /* The song's progress: a hairline that only ever moves forward. */
  .line {
    position: absolute;
    left: 18px;
    right: 18px;
    bottom: 3px;
    height: 2px;
    border-radius: 1px;
    background: var(--island-track);
    overflow: hidden;
  }
  .line i,
  .bar i {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--island-accent);
    transition: width 0.5s linear;
  }
  .peek {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 16px 0 10px;
    box-sizing: border-box;
  }
  .peek img,
  .peek .placeholder {
    border-radius: 10px;
  }
  .meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
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
    font-size: 12px;
    color: var(--island-fg-dim);
  }
  .bar {
    height: 3px;
    margin-top: 5px;
    border-radius: 2px;
    background: var(--island-track);
    overflow: hidden;
  }
  .right {
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
  }
  .time {
    font-size: 11px;
    color: var(--island-fg-dim);
    font-variant-numeric: tabular-nums;
  }
</style>
