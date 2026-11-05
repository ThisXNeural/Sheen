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
  import OsdView from "./lib/views/Osd.svelte";

  // Rendered before the first state arrives from Rust.
  let p = $state<Presentation>({
    mode: "hidden",
    geometry: { width: 0, height: 0, radius: 0 },
    hovering: false,
    media: null,
    osd: null,
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

  // Scroll on the capsule changes volume without opening anything.
  function onWheel(e: WheelEvent) {
    e.preventDefault();
    ipc.adjustVolume(e.deltaY < 0 ? 0.02 : -0.02);
  }

  function onClick() {
    if (p.mode === "mediaCompact") ipc.expand();
    // The idle pill is the island's "home button": it opens the main window.
    else if (p.mode === "idle") ipc.openMainWindow();
  }
</script>

<main>
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
    {#key p.mode}
      <div class="content" in:fade={{ duration: 180, delay: 90 }} out:fade={{ duration: 90 }}>
        {#if p.mode === "idle"}
          <Idle />
        {:else if p.mode === "mediaCompact" && p.media}
          <MediaCompact media={p.media} {artwork} />
        {:else if p.mode === "osd" && p.osd}
          <OsdView osd={p.osd} />
        {:else if p.mode === "expanded" && p.media}
          <MediaExpanded media={p.media} {artwork} osd={p.osd} />
        {/if}
      </div>
    {/key}
  </div>
</main>

<style>
  main {
    display: flex;
    justify-content: center;
    /* Must equal sheen_core::geometry::TOP_MARGIN. */
    padding-top: 8px;
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
