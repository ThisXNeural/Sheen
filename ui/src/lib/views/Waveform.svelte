<script lang="ts">
  // Placeholder visualiser: animated bars while playing. A real level meter
  // (IAudioMeterInformation peak values) is on the roadmap.
  let { playing }: { playing: boolean } = $props();
</script>

<div class="wave" class:playing aria-hidden="true">
  {#each [0.55, 1, 0.7, 0.9, 0.5] as h, i}
    <i style:--h={h} style:--d="{0.9 + i * 0.13}s"></i>
  {/each}
</div>

<style>
  .wave {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 14px;
  }
  i {
    width: 3px;
    height: 100%;
    border-radius: 2px;
    background: var(--island-accent);
    transform: scaleY(0.2);
    transition: transform 0.3s;
  }
  .playing i {
    animation: bounce var(--d) ease-in-out infinite alternate;
  }
  @keyframes bounce {
    from {
      transform: scaleY(0.2);
    }
    to {
      transform: scaleY(var(--h));
    }
  }
</style>
