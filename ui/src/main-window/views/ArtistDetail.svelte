<script lang="ts">
  import Icon from "../../shared/Icon.svelte";
  import { playAll } from "../actions";
  import { store } from "../store.svelte";
  import TrackList from "../TrackList.svelte";
  import AlbumGrid from "./AlbumGrid.svelte";
  import DetailHeader from "./DetailHeader.svelte";

  let { name }: { name: string } = $props();
  const artist = $derived(store.artistsByName.get(name));
  let tab = $state<"albums" | "songs">("albums");
</script>

{#if artist}
  <DetailHeader
    kind="Artist"
    title={artist.name}
    art={artist.art}
    round
    lines={[`${artist.albums.length} albums · ${artist.tracks.length} songs`]}
  >
    {#snippet actions()}
      <button class="pill accent" onclick={() => playAll(artist.tracks)}><Icon name="play" size={15} filled /> Play</button>
      <button class="pill" onclick={() => playAll(artist.tracks, 0, true)}><Icon name="shuffle" size={15} /> Shuffle</button>
    {/snippet}
  </DetailHeader>
  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={tab === "albums"} class:on={tab === "albums"} onclick={() => (tab = "albums")}>
      Albums
    </button>
    <button role="tab" aria-selected={tab === "songs"} class:on={tab === "songs"} onclick={() => (tab = "songs")}>
      Songs
    </button>
  </div>
  {#if tab === "albums"}
    <div class="scroll">
      {#if artist.albums.length}
        <AlbumGrid albums={artist.albums} showArtist={false} />
      {:else}
        <p class="missing">No albums — see Songs.</p>
      {/if}
    </div>
  {:else}
    <TrackList tracks={artist.tracks} sortable />
  {/if}
{:else}
  <p class="missing">This artist is no longer in your library.</p>
{/if}

<style>
  .tabs {
    display: flex;
    gap: 4px;
    padding: 0 24px 10px;
  }
  .tabs button {
    padding: 6px 14px;
    border-radius: 999px;
    color: var(--fg-dim);
    font-weight: 600;
  }
  .tabs button.on {
    background: var(--fg);
    color: var(--bg);
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
  }
  .missing {
    margin: 40px 24px;
    color: var(--fg-faint);
  }
</style>
