<script lang="ts">
  // Virtualised track table used by every view. Only the rows in view (plus
  // a margin) exist in the DOM, so a 20,000-track library scrolls as cheaply
  // as a 20-track album.
  import Cover from "../shared/Cover.svelte";
  import Icon from "../shared/Icon.svelte";
  import { formatTime } from "../shared/format";
  import { openTrackMenu, playAll } from "./actions";
  import * as api from "./api";
  import { store } from "./store.svelte";

  type SortKey = "title" | "artist" | "album" | "duration";

  let {
    tracks,
    numbering = "index",
    showAlbum = true,
    sortable = false,
    playlistId,
    empty = "No songs",
  }: {
    tracks: api.Track[];
    numbering?: "index" | "track";
    showAlbum?: boolean;
    sortable?: boolean;
    playlistId?: number;
    empty?: string;
  } = $props();

  const ROW = 52;
  const OVERSCAN = 6;

  let sort = $state<{ key: SortKey; dir: 1 | -1 } | null>(null);
  const shown = $derived.by(() => {
    if (!sort) return tracks;
    const { key, dir } = sort;
    const value = (t: api.Track) =>
      key === "duration" ? t.durationMs : key === "title" ? t.title : key === "artist" ? t.artist : t.album;
    return [...tracks].sort((a, b) => {
      const x = value(a);
      const y = value(b);
      return dir * (typeof x === "number" ? x - (y as number) : x.localeCompare(y as string));
    });
  });

  function toggleSort(key: SortKey) {
    if (!sortable) return;
    sort = sort?.key === key ? (sort.dir === 1 ? { key, dir: -1 } : null) : { key, dir: 1 };
  }

  let scrollTop = $state(0);
  let viewport = $state(0);
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN));
  const last = $derived(Math.min(shown.length, Math.ceil((scrollTop + viewport) / ROW) + OVERSCAN));
  const rows = $derived(shown.slice(first, last));

  // Selection: indexes into `shown`.
  let selected = $state(new Set<number>());
  let anchor = $state<number | null>(null);
  $effect(() => {
    void shown;
    selected = new Set();
    anchor = null;
  });

  function click(e: MouseEvent, i: number) {
    if (e.shiftKey && anchor != null) {
      const [a, b] = anchor < i ? [anchor, i] : [i, anchor];
      selected = new Set(Array.from({ length: b - a + 1 }, (_, k) => a + k));
    } else if (e.ctrlKey || e.metaKey) {
      const next = new Set(selected);
      if (next.has(i)) next.delete(i);
      else next.add(i);
      selected = next;
      anchor = i;
    } else {
      selected = new Set([i]);
      anchor = i;
    }
  }

  function context(e: MouseEvent, i: number) {
    if (!selected.has(i)) {
      selected = new Set([i]);
      anchor = i;
    }
    const chosen = [...selected].sort((a, b) => a - b).map((k) => shown[k]);
    openTrackMenu(e, chosen, { playlistId, list: shown, index: i });
  }

  let list = $state<HTMLDivElement>();
  function key(e: KeyboardEvent) {
    if (!shown.length) return;
    const i = anchor ?? -1;
    let next: number | null = null;
    if (e.key === "ArrowDown") next = Math.min(shown.length - 1, i + 1);
    else if (e.key === "ArrowUp") next = Math.max(0, i - 1);
    else if (e.key === "Enter" && anchor != null) return playAll(shown, anchor);
    else if (e.key === "a" && (e.ctrlKey || e.metaKey)) {
      selected = new Set(shown.map((_, k) => k));
      e.preventDefault();
      return;
    } else return;
    e.preventDefault();
    selected = new Set([next]);
    anchor = next;
    // Keep the selection in view.
    if (list) {
      const top = next * ROW;
      if (top < list.scrollTop) list.scrollTop = top;
      else if (top + ROW > list.scrollTop + list.clientHeight) list.scrollTop = top + ROW - list.clientHeight;
    }
  }

  const currentPath = $derived(store.player?.track?.path ?? null);
  const playing = $derived(store.player?.playing ?? false);
</script>

<div class="table" class:no-album={!showAlbum}>
  <div class="row head" role="row">
    <span class="num">#</span>
    <button class="sort" disabled={!sortable} onclick={() => toggleSort("title")}>
      Title{#if sort?.key === "title"}<Icon name={sort.dir === 1 ? "arrowUp" : "arrowDown"} size={12} />{/if}
    </button>
    {#if showAlbum}
      <button class="sort album" disabled={!sortable} onclick={() => toggleSort("album")}>
        Album{#if sort?.key === "album"}<Icon name={sort.dir === 1 ? "arrowUp" : "arrowDown"} size={12} />{/if}
      </button>
    {/if}
    <span></span>
    <button class="sort time" disabled={!sortable} onclick={() => toggleSort("duration")} aria-label="Duration">
      {#if sort?.key === "duration"}<Icon name={sort.dir === 1 ? "arrowUp" : "arrowDown"} size={12} />{/if}<Icon
        name="clock"
        size={14}
      />
    </button>
  </div>

  {#if shown.length === 0}
    <p class="empty">{empty}</p>
  {:else}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="list"
      tabindex="0"
      role="listbox"
      aria-label="Songs"
      aria-multiselectable="true"
      bind:this={list}
      bind:clientHeight={viewport}
      onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
      onkeydown={key}
    >
      <div class="spacer" style:height="{shown.length * ROW}px">
        {#each rows as track, k (track.id + ":" + (first + k))}
          {@const i = first + k}
          {@const current = track.path === currentPath}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            class="row"
            role="option"
            tabindex="-1"
            aria-selected={selected.has(i)}
            class:selected={selected.has(i)}
            class:current
            style:transform="translateY({i * ROW}px)"
            onclick={(e) => click(e, i)}
            ondblclick={() => playAll(shown, i)}
            oncontextmenu={(e) => context(e, i)}
          >
            <span class="num">
              {#if current && playing}
                <span class="bars" aria-label="Playing"><i></i><i></i><i></i></span>
              {:else}
                <span class="n">{numbering === "track" ? (track.trackNo ?? "–") : i + 1}</span>
                <button class="play" aria-label="Play {track.title}" onclick={() => playAll(shown, i)}>
                  <Icon name="play" size={14} filled />
                </button>
              {/if}
            </span>
            <span class="main">
              <Cover artKey={track.art} size={36} radius={7} />
              <span class="text">
                <span class="title" dir="auto">{track.title}</span>
                <span class="artist" dir="auto">{track.artist || "Unknown artist"}</span>
              </span>
            </span>
            {#if showAlbum}<span class="album" dir="auto">{track.album}</span>{/if}
            <button
              class="like"
              class:on={store.liked.has(track.path)}
              aria-label={store.liked.has(track.path) ? "Remove from Liked Songs" : "Add to Liked Songs"}
              onclick={(e) => {
                e.stopPropagation();
                api.toggleLike(track.path);
              }}
            >
              <Icon name="heart" size={15} filled={store.liked.has(track.path)} />
            </button>
            <span class="time">{formatTime(track.durationMs)}</span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .table {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    container-type: inline-size;
  }
  .row {
    display: grid;
    grid-template-columns: 40px minmax(0, 2fr) minmax(0, 1.2fr) 32px 52px;
    gap: 12px;
    align-items: center;
    padding: 0 20px 0 16px;
  }
  .no-album .row {
    grid-template-columns: 40px minmax(0, 1fr) 32px 52px;
  }
  @container (max-width: 560px) {
    .row {
      grid-template-columns: 40px minmax(0, 1fr) 32px 52px;
    }
    .row .album {
      display: none;
    }
  }
  .head {
    height: 32px;
    flex: none;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--fg-faint);
    border-bottom: 1px solid var(--border);
  }
  .sort {
    display: flex;
    align-items: center;
    gap: 4px;
    font: inherit;
    color: inherit;
    text-transform: inherit;
    letter-spacing: inherit;
    min-width: 0;
  }
  .sort:disabled {
    cursor: default;
  }
  .sort.time {
    justify-content: flex-end;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    outline: none;
  }
  .spacer {
    position: relative;
  }
  .list .row {
    position: absolute;
    left: 0;
    right: 0;
    height: 52px;
    color: var(--fg-dim);
    border-radius: 8px;
    margin: 0 6px;
    padding: 0 14px 0 10px;
  }
  .list .row:hover {
    background: var(--bg-hover);
  }
  .list .row.selected {
    background: var(--bg-active);
  }
  .num {
    position: relative;
    display: grid;
    place-items: center;
    height: 100%;
    font-variant-numeric: tabular-nums;
    color: var(--fg-faint);
  }
  .play {
    position: absolute;
    inset: 0;
    display: none;
    place-items: center;
    color: var(--fg);
  }
  .row:hover .play {
    display: grid;
  }
  .row:hover .n {
    visibility: hidden;
  }
  .main {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    gap: 1px;
  }
  .title,
  .artist,
  .album {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .title {
    color: var(--fg);
    font-weight: 500;
  }
  .artist {
    font-size: 12px;
  }
  .current .title,
  .current .n {
    color: var(--accent);
    font-weight: 600;
  }
  .like {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    color: var(--fg-faint);
    opacity: 0;
  }
  .row:hover .like,
  .like.on {
    opacity: 1;
  }
  .like.on {
    color: var(--danger);
  }
  .like:focus-visible {
    opacity: 1;
  }
  .time {
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--fg-faint);
  }
  .empty {
    margin: 40px auto;
    color: var(--fg-faint);
  }
  .bars {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 12px;
  }
  .bars i {
    width: 3px;
    height: 100%;
    border-radius: 1px;
    background: var(--accent);
    transform-origin: bottom;
    animation: eq 0.9s ease-in-out infinite alternate;
  }
  .bars i:nth-child(2) {
    animation-delay: -0.3s;
  }
  .bars i:nth-child(3) {
    animation-delay: -0.6s;
  }
  @keyframes eq {
    from {
      transform: scaleY(0.25);
    }
    to {
      transform: scaleY(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bars i {
      animation: none;
    }
  }
</style>
