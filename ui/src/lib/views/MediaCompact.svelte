<script lang="ts">
  import type { MediaInfo } from "../types";
  import Waveform from "./Waveform.svelte";

  let { media, artwork }: { media: MediaInfo; artwork: string | null } = $props();
</script>

<div class="compact">
  {#if artwork}
    <img src={artwork} alt="" />
  {:else}
    <div class="placeholder"></div>
  {/if}
  <span class="title">{media.title}</span>
  <Waveform playing={media.playing} />
</div>

<style>
  .compact {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px 0 6px;
    box-sizing: border-box;
  }
  img,
  .placeholder {
    width: 24px;
    height: 24px;
    border-radius: 7px;
    object-fit: cover;
    flex: none;
  }
  .placeholder {
    background: var(--island-track);
  }
  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-weight: 600;
  }
</style>
