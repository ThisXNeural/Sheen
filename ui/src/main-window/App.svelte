<script lang="ts">
  import { onMount } from "svelte";
  import { fly } from "svelte/transition";
  import { initTheme } from "../shared/theme";
  import * as api from "./api";
  import ContextMenu from "./ContextMenu.svelte";
  import { toast } from "./menu.svelte";
  import PlayerBar from "./PlayerBar.svelte";
  import PromptDialog from "./PromptDialog.svelte";
  import QueuePanel from "./QueuePanel.svelte";
  import { router } from "./router.svelte";
  import Sidebar from "./Sidebar.svelte";
  import { store } from "./store.svelte";
  import AlbumDetail from "./views/AlbumDetail.svelte";
  import Albums from "./views/Albums.svelte";
  import ArtistDetail from "./views/ArtistDetail.svelte";
  import Artists from "./views/Artists.svelte";
  import PlaylistView from "./views/PlaylistView.svelte";
  import Settings from "./views/Settings.svelte";
  import Songs from "./views/Songs.svelte";

  let queueOpen = $state(new URLSearchParams(location.search).has("queue"));

  onMount(() => {
    initTheme();
    store.init();
  });

  const typing = (t: EventTarget | null) =>
    t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement || (t as HTMLElement)?.isContentEditable;

  function keys(e: KeyboardEvent) {
    const mod = e.ctrlKey || e.metaKey;
    if (e.key === " " && !typing(e.target) && !(e.target instanceof HTMLButtonElement)) {
      e.preventDefault();
      api.command({ type: "togglePlay" });
    } else if (mod && e.key === "ArrowRight") {
      api.command({ type: "next" });
    } else if (mod && e.key === "ArrowLeft") {
      api.command({ type: "previous" });
    } else if (mod && (e.key === "ArrowUp" || e.key === "ArrowDown")) {
      e.preventDefault();
      const v = Math.min(1, Math.max(0, (store.player?.volume ?? 0.8) + (e.key === "ArrowUp" ? 0.05 : -0.05)));
      api.command({ type: "setVolume", volume: v });
    } else if (mod && e.key.toLowerCase() === "f") {
      e.preventDefault();
      if (!["songs", "albums", "artists"].includes(router.route.view)) router.root({ view: "songs" });
      queueMicrotask(() => document.querySelector<HTMLInputElement>("[data-search]")?.focus());
    } else if (mod && e.key.toLowerCase() === "l" && store.player?.track) {
      e.preventDefault();
      api.toggleLike(store.player.track.path);
    } else return;
  }
</script>

<svelte:window onkeydown={keys} />

<div class="shell" class:queue={queueOpen}>
  <Sidebar />

  <main>
    {#key JSON.stringify(router.route)}
      <div class="view">
        {#if router.route.view === "songs"}
          <Songs />
        {:else if router.route.view === "albums"}
          <Albums />
        {:else if router.route.view === "album"}
          <AlbumDetail key={router.route.key} />
        {:else if router.route.view === "artists"}
          <Artists />
        {:else if router.route.view === "artist"}
          <ArtistDetail name={router.route.name} />
        {:else if router.route.view === "liked"}
          <PlaylistView />
        {:else if router.route.view === "playlist"}
          <PlaylistView id={router.route.id} />
        {:else}
          <Settings />
        {/if}
      </div>
    {/key}
  </main>

  {#if queueOpen}
    <QueuePanel onclose={() => (queueOpen = false)} />
  {/if}

  <div class="bar"><PlayerBar bind:queueOpen /></div>
</div>

<ContextMenu />
<PromptDialog />
{#if toast.text}
  <div class="toast" role="status" transition:fly={{ y: 12, duration: 160 }}>{toast.text}</div>
{/if}

<style>
  .shell {
    height: 100%;
    display: grid;
    grid-template-columns: 210px minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr) auto;
  }
  .shell.queue {
    grid-template-columns: 210px minmax(0, 1fr) auto;
  }
  main {
    min-width: 0;
    min-height: 0;
    display: flex;
  }
  .view {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .bar {
    grid-column: 1 / -1;
  }
  .toast {
    position: fixed;
    left: 50%;
    bottom: 96px;
    transform: translateX(-50%);
    max-width: min(420px, calc(100vw - 32px));
    padding: 8px 14px;
    border-radius: 999px;
    background: var(--fg);
    color: var(--bg);
    font-weight: 500;
    box-shadow: var(--shadow);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    z-index: 40;
  }
  @media (max-width: 860px) {
    .shell.queue {
      grid-template-columns: 210px minmax(0, 1fr);
    }
    .shell.queue :global(aside) {
      position: fixed;
      right: 0;
      top: 0;
      bottom: 80px;
      z-index: 30;
      box-shadow: var(--shadow);
    }
  }
</style>
