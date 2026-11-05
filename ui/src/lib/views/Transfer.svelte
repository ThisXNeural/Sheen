<script lang="ts">
  import Icon from "../../shared/Icon.svelte";
  import { formatBytes, formatEta } from "../../shared/format";
  import type { Transfer } from "../types";
  import TransferLine from "./TransferLine.svelte";

  let { transfers, peek }: { transfers: Transfer[]; peek: boolean } = $props();

  const t = $derived(transfers[0]);
  const others = $derived(transfers.length - 1);
  const percent = $derived(t.progress != null ? `${Math.floor(t.progress * 100)}%` : formatBytes(t.doneBytes));
  // Most important first: time left, then speed. Sizes go on the right.
  const detail = $derived.by(() => {
    if (t.state === "done") return t.kind === "download" ? "Download complete" : "Done";
    if (t.state === "stalled") return "Waiting…";
    const parts = [];
    if (t.etaSecs != null) parts.push(formatEta(t.etaSecs));
    if (t.speedBps) parts.push(`${formatBytes(t.speedBps)}/s`);
    return parts.join(" · ") || t.app;
  });
  const sizes = $derived(
    t.doneBytes != null && t.totalBytes != null
      ? `${formatBytes(t.doneBytes)} / ${formatBytes(t.totalBytes)}`
      : t.doneBytes != null
        ? formatBytes(t.doneBytes)
        : t.app,
  );
</script>

{#if peek}
  <div class="peek">
    <div class="top">
      <span class="icon" class:done={t.state === "done"}>
        <Icon name={t.state === "done" ? "check" : t.kind === "download" ? "download" : "copy"} size={16} />
      </span>
      <span class="name" dir="auto">{t.name}</span>
      {#if others > 0}<span class="more">+{others}</span>{/if}
      <span class="pct">{percent}</span>
    </div>
    {#if t.progress != null}
      <div class="bar"><i style:width="{t.progress * 100}%"></i></div>
    {/if}
    <div class="detail">
      <span class="d">{detail}</span>
      <span class="app">{sizes}</span>
    </div>
  </div>
{:else}
  <div class="compact">
    <span class="icon" class:done={t.state === "done"}>
      <Icon name={t.state === "done" ? "check" : t.kind === "download" ? "download" : "copy"} size={15} />
    </span>
    <span class="name" dir="auto">{t.name}</span>
    {#if others > 0}<span class="more">+{others}</span>{/if}
    <span class="pct">{t.state === "done" ? "Done" : percent}</span>
    <TransferLine transfer={t} />
  </div>
{/if}

<style>
  .compact {
    position: relative;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 14px 0 12px;
    box-sizing: border-box;
  }
  .icon {
    flex: none;
    display: grid;
    place-items: center;
    color: var(--island-fg-dim);
  }
  .icon.done {
    color: #30d158;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-weight: 600;
  }
  .more {
    flex: none;
    font-size: 11px;
    color: var(--island-fg-dim);
  }
  .pct {
    flex: none;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    color: var(--island-fg-dim);
  }
  .peek {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 6px;
    padding: 0 20px;
    box-sizing: border-box;
  }
  .top,
  .detail {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .detail {
    font-size: 11px;
    color: var(--island-fg-dim);
    font-variant-numeric: tabular-nums;
  }
  .d {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .app {
    flex: none;
    max-width: 45%;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .bar {
    height: 4px;
    border-radius: 2px;
    background: var(--island-track);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--island-accent);
    transition: width 0.9s linear;
  }
</style>
