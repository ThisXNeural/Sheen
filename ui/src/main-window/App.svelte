<script lang="ts">
  import { onMount } from "svelte";
  import { initTheme } from "../shared/theme";
  import * as api from "./api";
  import Icon from "./Icon.svelte";
  import Library from "./Library.svelte";
  import PlayerBar from "./PlayerBar.svelte";
  import Settings from "./Settings.svelte";

  let view = $state<"library" | "settings">("library");
  let tracks = $state.raw<api.Track[]>([]);
  let status = $state<api.LibraryStatus>({ scanning: false, count: 0, root: null });
  let player = $state<api.PlayerState | null>(null);

  onMount(() => {
    initTheme();
    const reload = async () => (tracks = await api.libraryTracks());
    const subs = [
      api.onPlayer((s) => (player = s)),
      api.onLibrary((s) => {
        const finished = status.scanning && !s.scanning;
        status = s;
        if (finished) reload();
      }),
    ];
    reload();
    api.libraryStatus().then((s) => (status = s));
    api.playerState().then((s) => s && (player = s));
    return () => subs.forEach((s) => s.then((unlisten) => unlisten()));
  });
</script>

<div class="shell">
  <nav>
    <div class="brand"><span class="dot"></span>Sheen</div>
    <button class:active={view === "library"} onclick={() => (view = "library")}>
      <Icon name="library" /> Library
    </button>
    <button class:active={view === "settings"} onclick={() => (view = "settings")}>
      <Icon name="settings" /> Settings
    </button>
  </nav>

  <main>
    {#if view === "library"}
      <Library {tracks} {status} current={player?.track ?? null} onSettings={() => (view = "settings")} />
    {:else}
      <Settings {status} />
    {/if}
  </main>

  <PlayerBar {player} />
</div>

<style>
  .shell {
    height: 100%;
    display: grid;
    grid-template-columns: 200px 1fr;
    grid-template-rows: 1fr auto;
  }
  nav {
    grid-row: 1;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border);
    padding: 16px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    font-weight: 700;
    padding: 4px 10px 16px;
  }
  .dot {
    width: 22px;
    height: 10px;
    border-radius: 5px;
    background: var(--fg);
  }
  nav button {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: 8px;
    color: var(--fg-dim);
    text-align: left;
  }
  nav button:hover {
    background: var(--bg-hover);
  }
  nav button.active {
    background: var(--bg-active);
    color: var(--fg);
    font-weight: 600;
  }
  main {
    grid-row: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .shell > :global(:last-child) {
    grid-column: 1 / -1;
  }
</style>
