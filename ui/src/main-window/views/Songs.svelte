<script lang="ts">
  import Icon from "../../shared/Icon.svelte";
  import { playAll } from "../actions";
  import { router } from "../router.svelte";
  import { store } from "../store.svelte";
  import TrackList from "../TrackList.svelte";
  import ViewHeader from "../ViewHeader.svelte";

  let query = $state("");
  // Lower-cased haystacks are built once per library load, not per keystroke.
  const haystacks = $derived(
    store.tracks.map((t) => `${t.title}\u0000${t.artist}\u0000${t.album}`.toLowerCase()),
  );
  const visible = $derived.by(() => {
    const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
    if (!terms.length) return store.tracks;
    return store.tracks.filter((_, i) => terms.every((t) => haystacks[i].includes(t)));
  });
  const subtitle = $derived(
    store.status.scanning
      ? "Scanning…"
      : `${store.tracks.length.toLocaleString()} songs${query ? ` · ${visible.length.toLocaleString()} shown` : ""}`,
  );
</script>

<ViewHeader title="Songs" {subtitle} bind:search={query}>
  {#snippet actions()}
    {#if store.tracks.length}
      <button class="pill" onclick={() => playAll(visible, 0, true)}><Icon name="shuffle" size={15} /> Shuffle</button>
    {/if}
  {/snippet}
</ViewHeader>

{#if store.tracks.length === 0}
  <div class="empty">
    <Icon name="music" size={40} stroke={1.5} />
    {#if store.status.scanning}
      <p>Scanning your music…</p>
    {:else}
      <p>No music found{store.status.root ? ` in ${store.status.root}` : ""}.</p>
      <button class="primary" onclick={() => router.root({ view: "settings" })}>Choose a music folder</button>
    {/if}
  </div>
{:else}
  <TrackList tracks={visible} sortable empty="No songs match your search" />
{/if}

<style>
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 24px;
    text-align: center;
    color: var(--fg-faint);
  }
  .empty p {
    margin: 0;
    color: var(--fg-dim);
    overflow-wrap: anywhere;
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
