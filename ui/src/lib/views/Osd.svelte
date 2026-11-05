<script lang="ts">
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

<div class="osd" class:inline class:alert>
  <span class="label">{label}</span>
  {#if level !== null}
    <div class="track"><div class="fill" style:width="{Math.round(level * 100)}%"></div></div>
    <span class="value">{Math.round(level * 100)}</span>
  {/if}
</div>

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
  .value {
    width: 2.2em;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--island-fg-dim);
  }
</style>
