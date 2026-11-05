<script lang="ts">
  import type { Snippet } from "svelte";
  import Cover from "../../shared/Cover.svelte";
  import Icon from "../../shared/Icon.svelte";
  import { router } from "../router.svelte";

  let {
    kind,
    title,
    lines = [],
    art,
    round = false,
    actions,
  }: { kind: string; title: string; lines?: string[]; art: string | null; round?: boolean; actions?: Snippet } =
    $props();
</script>

<header>
  {#if router.canGoBack}
    <button class="back" aria-label="Back" onclick={() => router.back()}><Icon name="back" size={18} /></button>
  {/if}
  <Cover artKey={art} size={148} big radius={12} {round} />
  <div class="info">
    <span class="kind">{kind}</span>
    <h1 dir="auto">{title}</h1>
    {#each lines as line}<p dir="auto">{line}</p>{/each}
    <div class="actions">{@render actions?.()}</div>
  </div>
</header>

<style>
  header {
    position: relative;
    display: flex;
    align-items: flex-end;
    gap: 20px;
    padding: 22px 24px 18px;
    min-width: 0;
  }
  .back {
    position: absolute;
    left: 12px;
    top: 12px;
    z-index: 1;
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--bg-active);
    backdrop-filter: blur(8px);
  }
  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .kind {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-faint);
  }
  h1 {
    margin: 0;
    font-size: 26px;
    line-height: 1.2;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }
  p {
    margin: 0;
    color: var(--fg-dim);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 10px;
  }
</style>
