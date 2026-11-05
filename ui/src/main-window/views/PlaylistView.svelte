<script lang="ts">
  import Icon from "../../shared/Icon.svelte";
  import { formatTime } from "../../shared/format";
  import { openTrackMenu, playAll } from "../actions";
  import * as api from "../api";
  import { prompt } from "../menu.svelte";
  import { router } from "../router.svelte";
  import { store } from "../store.svelte";
  import TrackList from "../TrackList.svelte";
  import DetailHeader from "./DetailHeader.svelte";

  let { id = null }: { id?: number | null } = $props();

  const playlist = $derived(id == null ? null : store.collections.playlists.find((p) => p.id === id));
  const tracks = $derived(
    id == null ? store.resolve(store.collections.liked) : playlist ? store.resolve(playlist.tracks) : [],
  );
  const total = $derived(tracks.reduce((s, t) => s + t.durationMs, 0));

  async function rename() {
    if (!playlist) return;
    const name = await prompt.ask("Rename playlist", playlist.name, "Rename");
    if (name?.trim()) api.playlistRename(playlist.id, name);
  }

  function remove() {
    if (!playlist) return;
    api.playlistDelete(playlist.id);
    router.root({ view: "songs" });
  }
</script>

{#if id == null || playlist}
  <DetailHeader
    kind={id == null ? "Collection" : "Playlist"}
    title={id == null ? "Liked Songs" : playlist!.name}
    art={tracks[0]?.art ?? null}
    lines={[`${tracks.length} songs${tracks.length ? ` · ${formatTime(total)}` : ""}`]}
  >
    {#snippet actions()}
      <button class="pill accent" disabled={!tracks.length} onclick={() => playAll(tracks)}>
        <Icon name="play" size={15} filled /> Play
      </button>
      <button class="pill" disabled={!tracks.length} onclick={() => playAll(tracks, 0, true)}>
        <Icon name="shuffle" size={15} /> Shuffle
      </button>
      {#if playlist}
        <button class="pill icon" aria-label="Rename" onclick={rename}><Icon name="pencil" size={15} /></button>
        <button class="pill icon" aria-label="Delete playlist" onclick={remove}><Icon name="trash" size={15} /></button>
      {/if}
      {#if tracks.length}
        <button class="pill icon" aria-label="More" onclick={(e) => openTrackMenu(e, tracks)}>
          <Icon name="more" size={16} />
        </button>
      {/if}
    {/snippet}
  </DetailHeader>
  <TrackList
    {tracks}
    playlistId={playlist?.id}
    empty={id == null ? "Songs you like show up here. Tap ♡ on any song." : "Right-click songs anywhere to add them here."}
  />
{:else}
  <p class="missing">This playlist was deleted.</p>
{/if}

<style>
  .missing {
    margin: 40px 24px;
    color: var(--fg-faint);
  }
</style>
