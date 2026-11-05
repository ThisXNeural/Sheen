<script lang="ts">
  // A hairline progress bar along the bottom of a capsule.
  import type { Transfer } from "../types";

  let { transfer }: { transfer: Transfer } = $props();
</script>

<div class="line" class:indeterminate={transfer.progress == null} class:done={transfer.state === "done"}>
  <i style:width="{(transfer.progress ?? 0.3) * 100}%"></i>
</div>

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
  .indeterminate i {
    animation: slide 1.4s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
</style>
