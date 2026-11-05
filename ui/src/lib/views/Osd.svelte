<script lang="ts">
  import Icon from "../../shared/Icon.svelte";
  import type { Osd } from "../types";

  let { osd, inline = false }: { osd: Osd; inline?: boolean } = $props();

  const label = $derived.by(() => {
    switch (osd.kind) {
      case "volume":
        return osd.muted ? "Muted" : "Volume";
      case "brightness":
        return "Brightness";
      case "micMute":
        return osd.muted ? "Microphone off" : "Microphone on";
      case "capsLock":
        return osd.on ? "Caps Lock on" : "Caps Lock off";
      case "deviceBattery":
        return osd.name;
      case "clipboard":
        return osd.files > 1 ? `${osd.files} files copied` : osd.image ? "Image copied" : "Copied";
    }
  });

  // 0..1 for a fill bar, or null for a pure status pill.
  const level = $derived.by(() => {
    switch (osd.kind) {
      case "volume":
        return osd.muted ? 0 : osd.level;
      case "brightness":
        return osd.level;
      case "deviceBattery":
        return osd.percent / 100;
      default:
        return null;
    }
  });

  const alert = $derived(
    (osd.kind === "micMute" && osd.muted) || (osd.kind === "deviceBattery" && osd.percent <= 15),
  );
</script>

{#if osd.kind === "clipboard"}
  <div class="clip">
    {#if osd.image}
      <img src={osd.image} alt="" />
    {:else}
      <span class="clip-icon"><Icon name={osd.files ? "file" : "clipboard"} size={18} /></span>
    {/if}
    <div class="clip-text">
      <span class="label">{label}</span>
      {#if osd.text}<span class="preview" dir="auto">{osd.text}</span>{/if}
    </div>
  </div>
{:else}
<div class="osd" class:inline class:alert>
  <span class="label">{label}</span>
  {#if level !== null}
    <div class="track"><div class="fill" style:width="{Math.round(level * 100)}%"></div></div>
    <span class="value">{Math.round(level * 100)}</span>
  {/if}
</div>
{/if}

<style>
  .osd {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 18px;
    box-sizing: border-box;
  }
  .osd.inline {
    height: auto;
    padding: 0;
  }
  .label {
    font-weight: 600;
    white-space: nowrap;
  }
  .alert .label {
    color: var(--island-danger);
  }
  .track {
    flex: 1;
    height: 6px;
    border-radius: 3px;
    background: var(--island-track);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 3px;
    background: var(--island-accent);
    transition: width 120ms ease-out;
  }
  .clip {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 20px 0 12px;
    box-sizing: border-box;
  }
  .clip img,
  .clip-icon {
    flex: none;
    width: 42px;
    height: 42px;
    border-radius: 10px;
    object-fit: cover;
  }
  .clip-icon {
    display: grid;
    place-items: center;
    background: var(--island-track);
    color: var(--island-fg);
  }
  .clip-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .preview {
    font-size: 12px;
    line-height: 1.3;
    color: var(--island-fg-dim);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }
  .value {
    width: 2.2em;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--island-fg-dim);
  }
</style>
