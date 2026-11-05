<script lang="ts">
  import { onMount } from "svelte";
  import { Spring } from "svelte/motion";
  import { fade } from "svelte/transition";
  import * as ipc from "./lib/ipc";
  import { initTheme } from "./shared/theme";
  import type { Presentation } from "./lib/types";
  import Idle from "./lib/views/Idle.svelte";
  import MediaCompact from "./lib/views/MediaCompact.svelte";
  import MediaExpanded from "./lib/views/MediaExpanded.svelte";
  import Icon from "./shared/Icon.svelte";
  import OsdView from "./lib/views/Osd.svelte";
  import Shelf from "./lib/views/Shelf.svelte";
  import TransferView from "./lib/views/Transfer.svelte";

  // Rendered before the first state arrives from Rust.
  let p = $state<Presentation>({
    mode: "hidden",
    geometry: { width: 0, height: 0, radius: 0 },
    hovering: false,
    media: null,
    osd: null,
    transfers: [],
    shelf: [],
    dragging: false,
    shelfBadge: false,
    scale: 1,
  });
  let artwork = $state<string | null>(null);

  // Under-damped springs give the elastic, "liquid" morph between modes.
  // Geometry targets come from sheen-core, which also drives hit-testing, so
  // the clickable area always matches what is drawn once the spring settles.
  const opts = { stiffness: 0.16, damping: 0.62 };
  const width = new Spring(0, opts);
  const height = new Spring(0, opts);
  const radius = new Spring(0, opts);
  const opacity = new Spring(0, { stiffness: 0.2, damping: 0.9 });

  $effect(() => {
    const hidden = p.mode === "hidden";
    // Shrink towards a small pill rather than to a point when hiding.
    width.target = hidden ? 60 : p.geometry.width;
    height.target = hidden ? 20 : p.geometry.height;
    radius.target = hidden ? 10 : p.geometry.radius;
    opacity.target = hidden ? 0 : 1;
  });

  onMount(() => {
    initTheme();
    const subs = [ipc.onPresentation((next) => (p = next)), ipc.onArtwork((url) => (artwork = url))];
    ipc.currentPresentation().then((next) => next && (p = next));
    ipc.currentArtwork().then((url) => (artwork = url));
    return () => subs.forEach((s) => s.then((unlisten) => unlisten()));
  });

  // Scroll on the capsule changes volume without opening anything (the
  // shelf scrolls its items instead).
  function onWheel(e: WheelEvent) {
    if (p.mode === "shelf") return;
    e.preventDefault();
    ipc.adjustVolume(e.deltaY < 0 ? 0.02 : -0.02);
  }

  function onClick() {
    if (p.mode === "mediaCompact") ipc.expand();
    // The idle pill is the island's "home button": it opens the main window.
    else if (p.mode === "idle") ipc.openMainWindow();
  }

  // The key decides when the content cross-fades: compact and hover (peek)
  // variants of a mode are different layouts.
  const contentKey = $derived(
    p.mode === "transfer" || p.mode === "mediaCompact" ? `${p.mode}-${p.hovering}` : p.mode,
  );
  const visible = $derived(p.mode !== "hidden");
</script>

<main>
  <div class="stage" style:transform="scale({p.scale})">
  {#if visible && p.hovering}
    <button
      class="side close"
      aria-label="Hide the island (hover here to bring it back)"
      title="Hide — hover here to bring it back"
      transition:fade={{ duration: 120 }}
      onclick={() => ipc.dismiss()}><Icon name="x" size={12} stroke={2.6} /></button
    >
  {/if}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div
    class="capsule"
    class:interactive={p.mode === "mediaCompact" || p.mode === "idle"}
    style:width="{width.current}px"
    style:height="{height.current}px"
    style:border-radius="{radius.current}px"
    style:opacity={opacity.current}
    onwheel={onWheel}
    onclick={onClick}
  >
    {#key contentKey}
      <div class="content" in:fade={{ duration: 180, delay: 90 }} out:fade={{ duration: 90 }}>
        {#if p.mode === "idle"}
          <Idle />
        {:else if p.mode === "mediaCompact" && p.media}
          <MediaCompact media={p.media} {artwork} transfer={p.transfers[0] ?? null} peek={p.hovering} />
        {:else if p.mode === "transfer" && p.transfers.length}
          <TransferView transfers={p.transfers} peek={p.hovering} />
        {:else if p.mode === "shelf"}
          <Shelf items={p.shelf} dragging={p.dragging} />
        {:else if p.mode === "osd" && p.osd}
          <OsdView osd={p.osd} />
        {:else if p.mode === "expanded" && p.media}
          <MediaExpanded media={p.media} {artwork} osd={p.osd} />
        {/if}
      </div>
    {/key}
  </div>
  {#if p.shelfBadge}
    <button
      class="side badge"
      aria-label="Open the shelf ({p.shelf.length} files)"
      title="Shelf · {p.shelf.length}"
      transition:fade={{ duration: 160 }}
      onclick={() => ipc.shelfRequest()}
    >
      <Icon name="shelf" size={14} />
      <span class="count">{p.shelf.length}</span>
    </button>
  {/if}
  </div>
</main>

<style>
  main {
    display: flex;
    justify-content: center;
    /* Must equal sheen_core::geometry::TOP_MARGIN. */
    padding-top: 8px;
  }

  .stage {
    position: relative;
    /* Island size from settings, anchored at the top centre of the window
       (8 px above this box), the same way sheen-core scales the hit area. */
    transform-origin: 50% -8px;
  }

  /* Round buttons beside the capsule: close on the left, shelf on the right.
     Their room is reserved by sheen_core::geometry::SIDE (8 px gap + 30 px). */
  .side {
    position: absolute;
    top: 2px;
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--island-bg);
    color: var(--island-fg);
    box-shadow: var(--island-shadow);
  }
  .close {
    right: calc(100% + 8px);
    color: var(--island-fg-dim);
  }
  .close:hover {
    color: var(--island-fg);
  }
  .badge {
    left: calc(100% + 8px);
  }
  .badge .count {
    position: absolute;
    top: -3px;
    right: -3px;
    min-width: 14px;
    height: 14px;
    padding: 0 3px;
    box-sizing: border-box;
    border-radius: 7px;
    background: #0a84ff;
    color: #fff;
    font-size: 9px;
    font-weight: 700;
    line-height: 14px;
    text-align: center;
  }

  .capsule {
    position: relative;
    overflow: hidden;
    background: var(--island-bg);
    box-shadow: var(--island-shadow);
    color: var(--island-fg);
    transition:
      background-color 0.3s,
      color 0.3s;
    will-change: width, height, border-radius;
  }

  .capsule.interactive {
    cursor: pointer;
  }

  .content {
    position: absolute;
    inset: 0;
  }
</style>
