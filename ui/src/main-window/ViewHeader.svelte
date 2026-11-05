<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "../shared/Icon.svelte";
  import { router } from "./router.svelte";

  let {
    title,
    subtitle = "",
    search = $bindable<string | undefined>(undefined),
    actions,
  }: { title: string; subtitle?: string; search?: string; actions?: Snippet } = $props();
</script>

<header>
  {#if router.canGoBack}
    <button class="back" aria-label="Back" onclick={() => router.back()}><Icon name="back" size={18} /></button>
  {/if}
  <div class="titles">
    <h1 dir="auto">{title}</h1>
    {#if subtitle}<p>{subtitle}</p>{/if}
  </div>
  <div class="right">
    {@render actions?.()}
    {#if search !== undefined}
      <label class="search">
        <Icon name="search" size={15} />
        <input placeholder="Search" bind:value={search} spellcheck="false" data-search dir="auto" />
        {#if search}
          <button aria-label="Clear search" onclick={() => (search = "")}><Icon name="x" size={14} /></button>
        {/if}
      </label>
    {/if}
  </div>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 18px 24px 12px;
    min-width: 0;
  }
  .back {
    width: 30px;
    height: 30px;
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--bg-active);
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  h1 {
    margin: 0;
    font-size: 22px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  p {
    margin: 2px 0 0;
    color: var(--fg-dim);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px 0 10px;
    height: 32px;
    width: clamp(150px, 22vw, 240px);
    box-sizing: border-box;
    border-radius: 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    color: var(--fg-faint);
  }
  .search:focus-within {
    border-color: var(--accent);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: none;
    color: var(--fg);
  }
  .search button {
    display: grid;
    place-items: center;
    color: var(--fg-faint);
  }
</style>
