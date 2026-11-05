<script lang="ts">
  import Cover from "../../shared/Cover.svelte";
  import { router } from "../router.svelte";
  import { store } from "../store.svelte";
  import ViewHeader from "../ViewHeader.svelte";

  let query = $state("");
  const visible = $derived.by(() => {
    const q = query.toLowerCase().trim();
    return q ? store.artists.filter((a) => a.name.toLowerCase().includes(q)) : store.artists;
  });
</script>

<ViewHeader title="Artists" subtitle="{store.artists.length.toLocaleString()} artists" bind:search={query} />
<div class="scroll">
  <div class="grid">
    {#each visible as artist (artist.name)}
      <button class="card" onclick={() => router.go({ view: "artist", name: artist.name })}>
        <Cover artKey={artist.art} size="100%" big round />
        <span class="name" dir="auto">{artist.name}</span>
        <span class="sub">
          {artist.albums.length}
          {artist.albums.length === 1 ? "album" : "albums"} · {artist.tracks.length}
          {artist.tracks.length === 1 ? "song" : "songs"}
        </span>
      </button>
    {/each}
  </div>
  {#if !visible.length}
    <p class="empty">{store.artists.length ? "No artists match your search" : "No artists yet"}</p>
  {/if}
</div>

<style>
  .scroll {
    flex: 1;
    overflow-y: auto;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 20px 16px;
    padding: 4px 24px 24px;
  }
  .card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    min-width: 0;
    padding: 10px;
    border-radius: 12px;
    text-align: center;
    content-visibility: auto;
    contain-intrinsic-size: auto 200px;
  }
  .card:hover {
    background: var(--bg-hover);
  }
  .card > :global(.cover) {
    aspect-ratio: 1;
    height: auto !important;
    margin-bottom: 6px;
  }
  .name,
  .sub {
    max-width: 100%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .name {
    font-weight: 600;
  }
  .sub {
    font-size: 12px;
    color: var(--fg-dim);
  }
  .empty {
    margin: 40px 24px;
    text-align: center;
    color: var(--fg-faint);
  }
</style>
