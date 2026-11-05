<script lang="ts">
  import { formatTime } from "../shared/format";
  import * as api from "./api";
  import Icon from "./Icon.svelte";

  let {
    tracks,
    status,
    current,
    onSettings,
  }: {
    tracks: api.Track[];
    status: api.LibraryStatus;
    current: api.Track | null;
    onSettings: () => void;
  } = $props();

  let query = $state("");

  // Lower-cased haystacks are built once per library load, not per keystroke.
  const haystacks = $derived(
    tracks.map((t) => `${t.title}\u0000${t.artist}\u0000${t.album}`.toLowerCase()),
  );
  const visible = $derived.by(() => {
    const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
    if (!terms.length) return tracks;
    return tracks.filter((_, i) => terms.every((t) => haystacks[i].includes(t)));
  });

  // Virtualised list: only the rows in view (plus a margin) are in the DOM,
  // so a 20k-track library scrolls as cheaply as a 20-track one.
  const ROW = 40;
  const OVERSCAN = 8;
  let scrollTop = $state(0);
  let viewport = $state(0);
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN));
  const last = $derived(Math.min(visible.length, Math.ceil((scrollTop + viewport) / ROW) + OVERSCAN));
  const rows = $derived(visible.slice(first, last));

  let selected = $state<number | null>(null);

  function play(index: number) {
    api.playTracks(
      visible.map((t) => t.id),
      index,
    );
  }

  function onKey(e: KeyboardEvent) {
    if (!visible.length) return;
    const i = selected ?? -1;
    if (e.key === "ArrowDown") selected = Math.min(visible.length - 1, i + 1);
    else if (e.key === "ArrowUp") selected = Math.max(0, i - 1);
    else if (e.key === "Enter" && selected != null) play(selected);
    else return;
    e.preventDefault();
  }
</script>

<header>
  <div>
    <h1>Library</h1>
    <p>
      {#if status.scanning}Scanning…{:else}{tracks.length.toLocaleString()} tracks{/if}
      {#if query}· {visible.length.toLocaleString()} shown{/if}
    </p>
  </div>
  <label class="search">
    <Icon name="search" size={16} />
    <input placeholder="Search" bind:value={query} spellcheck="false" />
  </label>
</header>

{#if tracks.length === 0}
  <div class="empty">
    <Icon name="music" size={40} />
    {#if status.scanning}
      <p>Scanning your music…</p>
    {:else}
      <p>No music found{status.root ? ` in ${status.root}` : ""}.</p>
      <button class="primary" onclick={onSettings}>Choose a music folder</button>
    {/if}
  </div>
{:else}
  <div class="cols">
    <span>#</span><span>Title</span><span>Artist</span><span>Album</span><span class="time">Time</span>
  </div>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="list"
    tabindex="0"
    role="listbox"
    aria-label="Tracks"
    bind:clientHeight={viewport}
    onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
    onkeydown={onKey}
  >
    <div class="spacer" style:height="{visible.length * ROW}px">
      {#each rows as track, k (track.id)}
        {@const i = first + k}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="row"
          role="option"
          tabindex="-1"
          aria-selected={selected === i}
          class:selected={selected === i}
          class:playing={current?.path === track.path}
          style:transform="translateY({i * ROW}px)"
          onclick={() => (selected = i)}
          ondblclick={() => play(i)}
        >
          <span class="num">{i + 1}</span>
          <span class="title">{track.title}</span>
          <span>{track.artist}</span>
          <span>{track.album}</span>
          <span class="time">{formatTime(track.durationMs)}</span>
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    padding: 20px 24px 12px;
  }
  h1 {
    margin: 0;
    font-size: 22px;
  }
  header p {
    margin: 2px 0 0;
    color: var(--fg-dim);
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    height: 32px;
    width: 240px;
    border-radius: 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    color: var(--fg-faint);
  }
  .search:focus-within {
    border-color: var(--accent);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: none;
    color: var(--fg);
  }
  .cols,
  .row {
    display: grid;
    grid-template-columns: 44px 2fr 1.3fr 1.3fr 64px;
    gap: 12px;
    align-items: center;
    padding: 0 24px;
  }
  .cols {
    height: 30px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--fg-faint);
    border-bottom: 1px solid var(--border);
  }
  .list {
    flex: 1;
    overflow-y: auto;
    outline: none;
  }
  .spacer {
    position: relative;
  }
  .row {
    position: absolute;
    left: 0;
    right: 0;
    height: 40px;
    color: var(--fg-dim);
  }
  .row span {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .row:hover {
    background: var(--bg-hover);
  }
  .row.selected {
    background: var(--bg-active);
  }
  .row .title {
    color: var(--fg);
    font-weight: 500;
  }
  .row.playing .title,
  .row.playing .num {
    color: var(--accent);
    font-weight: 600;
  }
  .num,
  .time {
    font-variant-numeric: tabular-nums;
    color: var(--fg-faint);
  }
  .time {
    text-align: right;
  }
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--fg-faint);
  }
  .empty p {
    margin: 0;
    color: var(--fg-dim);
  }
  .primary {
    margin-top: 6px;
    padding: 7px 14px;
    border-radius: 8px;
    background: var(--accent);
    color: var(--accent-fg);
    font-weight: 600;
  }
</style>
