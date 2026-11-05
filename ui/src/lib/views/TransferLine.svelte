<script lang="ts">
  // A hairline progress bar along the bottom of a capsule. Drawn only when
  // the progress is known: an endlessly sliding bar is just noise.
  import type { Transfer } from "../types";

  let { transfer }: { transfer: Transfer } = $props();
</script>

{#if transfer.progress != null}
  <div class="line" class:done={transfer.state === "done"}>
    <i style:width="{transfer.progress * 100}%"></i>
  </div>
{/if}

<style>
  .line {
    position: absolute;
    left: 14%;
    right: 14%;
    bottom: 3px;
    height: 2px;
    border-radius: 1px;
    background: var(--island-track);
    overflow: hidden;
  }
  i {
    display: block;
    height: 100%;
    border-radius: 1px;
    background: var(--island-accent);
    transition: width 0.9s linear;
  }
  .done i {
    background: #30d158;
  }
</style>
