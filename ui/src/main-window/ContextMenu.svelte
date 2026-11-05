<script lang="ts">
  import Icon from "../shared/Icon.svelte";
  import { menu, type MenuItem } from "./menu.svelte";

  let root = $state<HTMLDivElement>();
  let sub = $state<{ items: MenuItem[]; x: number; y: number } | null>(null);
  let subEl = $state<HTMLDivElement>();

  // Keep the menu (and its submenu) inside the window.
  let pos = $state({ x: 0, y: 0 });
  $effect(() => {
    if (!menu.open || !root) return;
    const r = root.getBoundingClientRect();
    pos = {
      x: Math.max(4, Math.min(menu.x, innerWidth - r.width - 4)),
      y: Math.max(4, Math.min(menu.y, innerHeight - r.height - 4)),
    };
  });
  let subPos = $state({ x: 0, y: 0 });
  $effect(() => {
    if (!sub || !subEl) return;
    const r = subEl.getBoundingClientRect();
    const right = sub.x + r.width < innerWidth - 4;
    subPos = {
      x: right ? sub.x : Math.max(4, sub.x - r.width - (root?.offsetWidth ?? 0) + 8),
      y: Math.max(4, Math.min(sub.y, innerHeight - r.height - 4)),
    };
  });

  function run(item: MenuItem) {
    if (item.disabled || item.submenu) return;
    menu.close();
    sub = null;
    item.action?.();
  }

  function enter(item: MenuItem, e: MouseEvent) {
    if (!item.submenu) {
      sub = null;
      return;
    }
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    sub = { items: item.submenu, x: r.right - 4, y: r.top - 5 };
  }

  $effect(() => {
    if (!menu.open) sub = null;
  });
</script>

<svelte:window
  onkeydown={(e) => e.key === "Escape" && menu.close()}
  onblur={() => menu.close()}
  onresize={() => menu.close()}
/>

{#if menu.open}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim" onclick={() => menu.close()} oncontextmenu={(e) => (e.preventDefault(), menu.close())}></div>
  <div class="menu" role="menu" bind:this={root} style:left="{pos.x}px" style:top="{pos.y}px">
    {#each menu.items as item}
      {#if item.separator}
        <div class="sep"></div>
      {:else}
        <button
          role="menuitem"
          class:danger={item.danger}
          disabled={item.disabled}
          onclick={() => run(item)}
          onmouseenter={(e) => enter(item, e)}
        >
          <span class="icon">{#if item.icon}<Icon name={item.icon} size={15} />{/if}</span>
          <span class="label">{item.label}</span>
          {#if item.submenu}<Icon name="chevronRight" size={14} />{/if}
        </button>
      {/if}
    {/each}
  </div>
  {#if sub}
    <div class="menu sub" role="menu" bind:this={subEl} style:left="{subPos.x}px" style:top="{subPos.y}px">
      {#each sub.items as item}
        {#if item.separator}
          <div class="sep"></div>
        {:else}
          <button role="menuitem" onclick={() => run(item)}>
            <span class="icon">{#if item.icon}<Icon name={item.icon} size={15} />{/if}</span>
            <span class="label" dir="auto">{item.label}</span>
          </button>
        {/if}
      {/each}
    </div>
  {/if}
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 50;
  }
  .menu {
    position: fixed;
    z-index: 51;
    min-width: 200px;
    max-width: 280px;
    max-height: calc(100vh - 8px);
    overflow-y: auto;
    padding: 5px;
    border-radius: 10px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    box-shadow: var(--shadow);
  }
  .menu button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 8px;
    border-radius: 6px;
    text-align: left;
  }
  .menu button:hover:not(:disabled) {
    background: var(--bg-active);
  }
  .menu button:disabled {
    opacity: 0.4;
  }
  .icon {
    width: 15px;
    flex: none;
    color: var(--fg-dim);
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .danger {
    color: var(--danger);
  }
  .danger .icon {
    color: var(--danger);
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--border);
  }
</style>
