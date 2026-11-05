<script lang="ts">
  import { icons, type IconName } from "./icons";

  let {
    name,
    size = 18,
    filled = false,
    stroke = 2,
  }: { name: IconName; size?: number; filled?: boolean; stroke?: number } = $props();
</script>

<svg
  class="icon"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={filled ? "currentColor" : "none"}
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  data-icon={name}>{@html icons[name]}</svg
>

<style>
  svg {
    display: block;
    flex: none;
  }
</style>
