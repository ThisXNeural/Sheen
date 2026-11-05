<script lang="ts">
  import Cover from "../../shared/Cover.svelte";
  import Icon from "../../shared/Icon.svelte";
  import { openTrackMenu, playAll } from "../actions";
  import { router } from "../router.svelte";
  import type { Album } from "../store.svelte";

  let { albums, showArtist = true }: { albums: Album[]; showArtist?: boolean } = $props();
</script>

<div class="grid">
  {#each albums as album (album.key)}
    <div class="card">
      <button
        class="art"
        aria-label="Open {album.title}"
        onclick={() => router.go({ view: "album", key: album.key })}
        oncontextmenu={(e) => openTrackMenu(e, album.tracks)}
      >
        <Cover artKey={album.key} size="100%" big radius={10} />
      </button>
      <button class="play" aria-label="Play {album.title}" onclick={() => playAll(album.tracks)}>
        <Icon name="play" size={16} filled />
      </button>
      <button class="meta" onclick={() => router.go({ view: "album", key: album.key })}>
        <span class="title" dir="auto">{album.title}</span>
        <span class="sub" dir="auto">
          {[showArtist ? album.artist : "", album.year ?? ""].filter(Boolean).join(" · ")}
        </span>
      </button>
    </div>
  {/each}
</div>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 20px 18px;
    padding: 4px 24px 24px;
  }
  .card {
    position: relative;
    min-width: 0;
    /* Skip layout and paint for cards far off screen. */
    content-visibility: auto;
    contain-intrinsic-size: auto 260px;
  }
  .art {
    display: block;
    width: 100%;
    aspect-ratio: 1;
    border-radius: 10px;
    overflow: hidden;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.12);
    transition: transform 0.15s;
  }
  .art:hover {
    transform: translateY(-2px);
  }
  .play {
    position: absolute;
    right: 8px;
    top: calc(100% - 8px - 40px - 48px);
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-fg);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
    opacity: 0;
    transform: translateY(6px);
    transition:
      opacity 0.15s,
      transform 0.15s;
  }
  .card:hover .play,
  .play:focus-visible {
    opacity: 1;
    transform: none;
  }
  .meta {
    display: flex;
    flex-direction: column;
    width: 100%;
    min-width: 0;
    height: 40px;
    margin-top: 8px;
    text-align: left;
  }
  .title,
  .sub {
    max-width: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .title {
    font-weight: 600;
  }
  .sub {
    color: var(--fg-dim);
    font-size: 12px;
  }
</style>
