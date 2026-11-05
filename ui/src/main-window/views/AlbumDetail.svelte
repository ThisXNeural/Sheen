<script lang="ts">
  import Icon from "../../shared/Icon.svelte";
  import { formatTime } from "../../shared/format";
  import { openTrackMenu, playAll } from "../actions";
  import { router } from "../router.svelte";
  import { store } from "../store.svelte";
  import TrackList from "../TrackList.svelte";
  import DetailHeader from "./DetailHeader.svelte";

  let { key }: { key: string } = $props();
  const album = $derived(store.albumsByKey.get(key));
</script>

{#if album}
  <DetailHeader
    kind="Album"
    title={album.title}
    art={album.key}
    lines={[
      [album.artist, album.year].filter(Boolean).join(" · "),
      `${album.tracks.length} songs · ${formatTime(album.durationMs)}`,
    ]}
  >
    {#snippet actions()}
      <button class="pill accent" onclick={() => playAll(album.tracks)}><Icon name="play" size={15} filled /> Play</button>
      <button class="pill" onclick={() => playAll(album.tracks, 0, true)}><Icon name="shuffle" size={15} /> Shuffle</button>
      <button class="pill" onclick={() => router.go({ view: "artist", name: album.artist })}>
        <Icon name="artists" size={15} /> Artist
      </button>
      <button class="pill icon" aria-label="More" onclick={(e) => openTrackMenu(e, album.tracks)}>
        <Icon name="more" size={16} />
      </button>
    {/snippet}
  </DetailHeader>
  <TrackList tracks={album.tracks} numbering="track" showAlbum={false} />
{:else}
  <p class="missing">This album is no longer in your library.</p>
{/if}

<style>
  .missing {
    margin: 40px 24px;
    color: var(--fg-faint);
  }
</style>
