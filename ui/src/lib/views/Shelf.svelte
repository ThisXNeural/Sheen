<script lang="ts">
  import { SvelteSet } from "svelte/reactivity";
  import { artUrl } from "../../shared/art";
  import Icon from "../../shared/Icon.svelte";
  import * as ipc from "../ipc";
  import type { ShelfItem } from "../types";

  let { items, dragging }: { items: ShelfItem[]; dragging: boolean } = $props();

  // Start a real shell drag once the pointer has moved a few pixels with the
  // button held, so a plain click still works.
  let pressed: { id: number; x: number; y: number } | null = null;
  function down(e: PointerEvent, id: number) {
    if (e.button === 0) pressed = { id, x: e.clientX, y: e.clientY };
  }
  function move(e: PointerEvent) {
    if (!pressed || !(e.buttons & 1)) return (pressed = null);
    if (Math.hypot(e.clientX - pressed.x, e.clientY - pressed.y) > 4) {
      ipc.shelfDragOut([pressed.id]);
      pressed = null;
    }
  }
  const failed = new SvelteSet<number>();
</script>

<svelte:window onpointerup={() => (pressed = null)} />

<div class="shelf" class:dragging>
  <div class="head">
    <span class="title"><Icon name="shelf" size={15} /> Shelf</span>
    {#if items.length}
      <span class="count">{items.length} {items.length === 1 ? "item" : "items"}</span>
      <button class="clear" onclick={() => ipc.shelfClear()}>Clear</button>
    {/if}
  </div>

  {#if dragging && !items.length}
    <div class="drop"><Icon name="download" size={18} /> Drop files to park them here</div>
  {:else if !items.length}
    <div class="drop quiet">Drag files onto the island to keep them handy</div>
  {:else}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="items" onpointermove={move}>
      {#each items as item (item.id)}
        <div
          class="item"
          role="button"
          tabindex="0"
          title={item.path}
          onpointerdown={(e) => down(e, item.id)}
          ondblclick={() => ipc.shelfOpen(item.id)}
          onkeydown={(e) => e.key === "Enter" && ipc.shelfOpen(item.id)}
        >
          <div class="thumb">
            {#if item.kind === "image" && artUrl("shelf", item.id) && !failed.has(item.id)}
              <img src={artUrl("shelf", item.id)} alt="" draggable="false" onerror={() => failed.add(item.id)} />
            {:else if item.kind === "folder"}
              <Icon name="folder" size={26} stroke={1.5} />
            {:else if item.kind === "image"}
              <Icon name="image" size={26} stroke={1.5} />
            {:else}
              <Icon name="file" size={26} stroke={1.5} />
              {#if item.ext}<span class="ext">{item.ext.slice(0, 4)}</span>{/if}
            {/if}
          </div>
          <span class="name" dir="auto">{item.name}</span>
          <button
            class="remove"
            aria-label="Remove {item.name}"
            onpointerdown={(e) => e.stopPropagation()}
            onclick={() => ipc.shelfRemove([item.id])}><Icon name="x" size={11} stroke={2.5} /></button
          >
        </div>
      {/each}
      {#if dragging}
        <div class="item add"><Icon name="plus" size={22} /></div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .shelf {
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 12px 16px 12px;
    box-sizing: border-box;
    gap: 8px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 600;
  }
  .count {
    flex: 1;
    font-size: 11px;
    color: var(--island-fg-dim);
  }
  .clear {
    font-size: 11px;
    font-weight: 600;
    color: var(--island-fg-dim);
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--island-track);
  }
  .drop {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    border: 1.5px dashed var(--island-fg-dim);
    border-radius: 16px;
    color: var(--island-fg);
    font-weight: 500;
    text-align: center;
    padding: 0 12px;
  }
  .drop.quiet {
    border-color: var(--island-track);
    color: var(--island-fg-dim);
    font-size: 12px;
  }
  .dragging .drop {
    animation: pulse 1.2s ease-in-out infinite alternate;
  }
  @keyframes pulse {
    to {
      border-color: var(--island-fg);
    }
  }
  .items {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 6px;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
  }
  .items::-webkit-scrollbar {
    display: none;
  }
  .item {
    position: relative;
    flex: none;
    width: 64px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 4px 2px 0;
    border-radius: 10px;
    cursor: grab;
  }
  .item:hover {
    background: var(--island-track);
  }
  .thumb {
    position: relative;
    width: 44px;
    height: 44px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    overflow: hidden;
    color: var(--island-fg-dim);
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .ext {
    position: absolute;
    bottom: 5px;
    font-size: 8px;
    font-weight: 800;
    text-transform: uppercase;
    color: var(--island-fg);
  }
  .name {
    max-width: 100%;
    font-size: 10px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--island-fg-dim);
  }
  .remove {
    position: absolute;
    top: 0;
    right: 2px;
    width: 16px;
    height: 16px;
    display: none;
    place-items: center;
    border-radius: 50%;
    background: var(--island-fg-dim);
    color: var(--island-bg);
  }
  .item:hover .remove,
  .remove:focus-visible {
    display: grid;
  }
  .add {
    justify-content: center;
    border: 1.5px dashed var(--island-fg-dim);
    color: var(--island-fg-dim);
    height: 62px;
    box-sizing: border-box;
    cursor: default;
  }
</style>
