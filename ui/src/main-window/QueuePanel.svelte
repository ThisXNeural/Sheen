<script lang="ts">
  import Cover from "../shared/Cover.svelte";
  import Icon from "../shared/Icon.svelte";
  import { formatTime } from "../shared/format";
  import { openTrackMenu } from "./actions";
  import * as api from "./api";
  import { store } from "./store.svelte";

  let { onclose }: { onclose: () => void } = $props();

  /** Show this many upcoming tracks; long queues add a "more" line. */
  const LIMIT = 150;

  let snapshot = $state<api.QueueSnapshot>({ tracks: [], index: 0 });
  // Refetch when the queue or the current track changes.
  $effect(() => {
    void store.player?.queueRev;
    void store.player?.track?.path;
    api.queueSnapshot().then((s) => (snapshot = s));
  });

  const current = $derived(snapshot.tracks[snapshot.index] ?? null);
  const upcoming = $derived(snapshot.tracks.slice(snapshot.index + 1, snapshot.index + 1 + LIMIT));
  const more = $derived(Math.max(0, snapshot.tracks.length - snapshot.index - 1 - LIMIT));
</script>

<aside aria-label="Queue">
  <header>
    <h2>Queue</h2>
    <button aria-label="Close queue" onclick={onclose}><Icon name="x" size={16} /></button>
  </header>
  <div class="scroll">
    {#if current}
      <h3>Now playing</h3>
      <div class="item now">
        <Cover artKey={current.art} size={40} radius={7} />
        <span class="text">
          <span class="title" dir="auto">{current.title}</span>
          <span class="artist" dir="auto">{current.artist}</span>
        </span>
        <span class="time">{formatTime(current.durationMs)}</span>
      </div>
    {/if}
    <h3>Up next</h3>
    {#each upcoming as track, k (k + ":" + track.id)}
      {@const index = snapshot.index + 1 + k}
      <div class="item" role="listitem" oncontextmenu={(e) => openTrackMenu(e, [track])}>
        <button class="jump" onclick={() => api.command({ type: "jumpTo", index })}>
          <Cover artKey={track.art} size={40} radius={7} />
          <span class="text">
            <span class="title" dir="auto">{track.title}</span>
            <span class="artist" dir="auto">{track.artist}</span>
          </span>
        </button>
        <button class="remove" aria-label="Remove from queue" onclick={() => api.command({ type: "removeAt", index })}>
          <Icon name="x" size={14} />
        </button>
      </div>
    {:else}
      <p class="empty">Nothing queued. Use “Play next” or “Add to queue” on any song.</p>
    {/each}
    {#if more}<p class="empty">and {more.toLocaleString()} more</p>{/if}
  </div>
</aside>

<style>
  aside {
    display: flex;
    flex-direction: column;
    min-height: 0;
    width: 300px;
    border-left: 1px solid var(--border);
    background: var(--bg);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 18px 14px 6px 18px;
  }
  header button {
    width: 28px;
    height: 28px;
    display: grid;
    place-items: center;
    border-radius: 50%;
  }
  header button:hover {
    background: var(--bg-hover);
  }
  h2 {
    margin: 0;
    font-size: 17px;
  }
  h3 {
    margin: 14px 18px 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--fg-faint);
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
    padding-bottom: 12px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 8px;
    padding: 6px 8px;
    border-radius: 8px;
    min-width: 0;
  }
  .item:hover {
    background: var(--bg-hover);
  }
  .jump {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title,
  .artist {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .title {
    font-weight: 500;
  }
  .now .title {
    color: var(--accent);
    font-weight: 600;
  }
  .artist {
    font-size: 12px;
    color: var(--fg-dim);
  }
  .time {
    flex: none;
    font-size: 12px;
    color: var(--fg-faint);
    font-variant-numeric: tabular-nums;
  }
  .remove {
    flex: none;
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    color: var(--fg-faint);
    opacity: 0;
  }
  .item:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }
  .remove:hover {
    background: var(--bg-active);
    color: var(--fg);
  }
  .empty {
    margin: 8px 18px;
    color: var(--fg-faint);
    font-size: 12px;
  }
</style>
