<script lang="ts">
  import Icon from "../../shared/Icon.svelte";

  let { shelfCount = 0 }: { shelfCount?: number } = $props();
</script>

<div class="idle">
  {#if shelfCount > 0}
    <span class="badge" aria-label="{shelfCount} files on the shelf">
      <Icon name="shelf" size={14} />
      {shelfCount}
    </span>
  {:else}
    <span class="dot"></span>
  {/if}
</div>

<style>
  .idle {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--island-fg-dim);
    opacity: 0.5;
  }
  .badge {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    font-weight: 600;
    color: var(--island-fg-dim);
    font-variant-numeric: tabular-nums;
  }
</style>
