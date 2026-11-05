<script lang="ts">
  import { store } from "../store.svelte";
  import ViewHeader from "../ViewHeader.svelte";
  import AlbumGrid from "./AlbumGrid.svelte";

  let query = $state("");
  const visible = $derived.by(() => {
    const q = query.toLowerCase().trim();
    if (!q) return store.albums;
    return store.albums.filter((a) => a.title.toLowerCase().includes(q) || a.artist.toLowerCase().includes(q));
  });
</script>

<ViewHeader title="Albums" subtitle="{store.albums.length.toLocaleString()} albums" bind:search={query} />
<div class="scroll">
  {#if visible.length}
    <AlbumGrid albums={visible} />
  {:else}
    <p class="empty">{store.albums.length ? "No albums match your search" : "No albums yet"}</p>
  {/if}
</div>

<style>
  .scroll {
    flex: 1;
    overflow-y: auto;
  }
  .empty {
    margin: 40px 24px;
    text-align: center;
    color: var(--fg-faint);
  }
</style>
