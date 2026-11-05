<script lang="ts">
  // Album art with a tinted placeholder underneath: the placeholder shows
  // until the image has decoded (or forever, when there is no art), so rows
  // never jump and there is no broken-image icon.
  import { artUrl, hueFor } from "./art";
  import Icon from "./Icon.svelte";

  let {
    artKey,
    size = 40,
    big = false,
    radius = 8,
    round = false,
  }: { artKey: string | null | undefined; size?: number | string; big?: boolean; radius?: number; round?: boolean } =
    $props();

  const src = $derived(artUrl(big ? "cover" : "thumb", artKey));
  const hue = $derived(hueFor(artKey ?? ""));
  let loaded = $state(false);
  let failedFor = $state<string | null>(null);
  $effect(() => {
    void src;
    loaded = false;
  });
  const css = $derived(typeof size === "number" ? `${size}px` : size);
  const glyph = $derived(typeof size === "number" ? Math.max(12, Math.round(size * 0.4)) : 40);
</script>

<div
  class="cover"
  style:width={css}
  style:height={css}
  style:border-radius={round ? "50%" : `${radius}px`}
  style:--hue={hue}
>
  {#if !loaded}
    <span class="glyph"><Icon name="music" size={glyph} stroke={1.75} /></span>
  {/if}
  {#if src && failedFor !== src}
    <img
      {src}
      alt=""
      loading="lazy"
      decoding="async"
      class:loaded
      onload={() => (loaded = true)}
      onerror={() => (failedFor = src)}
    />
  {/if}
</div>

<style>
  .cover {
    position: relative;
    flex: none;
    overflow: hidden;
    display: grid;
    place-items: center;
    background: linear-gradient(
      135deg,
      hsl(var(--hue) 45% 62% / 0.55),
      hsl(calc(var(--hue) + 40) 40% 42% / 0.55)
    );
    color: rgba(255, 255, 255, 0.85);
  }
  .glyph {
    display: grid;
    place-items: center;
  }
  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity 0.2s;
  }
  img.loaded {
    opacity: 1;
  }
</style>
