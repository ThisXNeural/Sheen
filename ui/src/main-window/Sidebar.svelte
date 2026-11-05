<script lang="ts">
  import Icon from "../shared/Icon.svelte";
  import type { IconName } from "../shared/icons";
  import { newPlaylist } from "./actions";
  import { router, type Route } from "./router.svelte";
  import { store } from "./store.svelte";

  const library: { route: Route; label: string; icon: IconName; match: string[] }[] = [
    { route: { view: "songs" }, label: "Songs", icon: "songs", match: ["songs"] },
    { route: { view: "albums" }, label: "Albums", icon: "albums", match: ["albums", "album"] },
    { route: { view: "artists" }, label: "Artists", icon: "artists", match: ["artists", "artist"] },
    { route: { view: "liked" }, label: "Liked Songs", icon: "heart", match: ["liked"] },
  ];
</script>

<nav aria-label="Library">
  <div class="brand"><span class="dot"></span>Sheen</div>
  <div class="section">
    {#each library as item}
      <button class:active={item.match.includes(router.route.view)} onclick={() => router.root(item.route)}>
        <Icon name={item.icon} size={17} />
        <span>{item.label}</span>
      </button>
    {/each}
  </div>

  <div class="heading">
    <span>Playlists</span>
    <button class="add" aria-label="New playlist" onclick={() => newPlaylist()}><Icon name="plus" size={15} /></button>
  </div>
  <div class="section playlists">
    {#each store.collections.playlists as p (p.id)}
      <button
        class:active={router.route.view === "playlist" && router.route.id === p.id}
        onclick={() => router.root({ view: "playlist", id: p.id })}
      >
        <Icon name="playlists" size={17} />
        <span dir="auto">{p.name}</span>
      </button>
    {:else}
      <p class="none">No playlists yet</p>
    {/each}
  </div>

  <div class="bottom">
    <button class:active={router.route.view === "settings"} onclick={() => router.root({ view: "settings" })}>
      <Icon name="settings" size={17} />
      <span>Settings</span>
    </button>
  </div>
</nav>

<style>
  nav {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    padding: 14px 10px 10px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    font-weight: 700;
    padding: 4px 10px 14px;
  }
  .dot {
    width: 22px;
    height: 10px;
    border-radius: 5px;
    background: var(--fg);
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .playlists {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 6px 6px 10px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-faint);
  }
  .add {
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 6px;
    color: var(--fg-dim);
  }
  .add:hover {
    background: var(--bg-hover);
  }
  .section button,
  .bottom button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-width: 0;
    padding: 7px 10px;
    border-radius: 8px;
    color: var(--fg-dim);
    text-align: left;
  }
  .section button span,
  .bottom button span {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .section button:hover,
  .bottom button:hover {
    background: var(--bg-hover);
  }
  .section button.active,
  .bottom button.active {
    background: var(--bg-active);
    color: var(--fg);
    font-weight: 600;
  }
  .none {
    margin: 4px 10px;
    color: var(--fg-faint);
    font-size: 12px;
  }
  .bottom {
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }
</style>
