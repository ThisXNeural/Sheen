<script lang="ts">
  import { onMount } from "svelte";
  import { setTheme, type ThemePref } from "../shared/theme";
  import * as api from "./api";

  let { status }: { status: api.LibraryStatus } = $props();

  let settings = $state<api.Settings | null>(null);

  onMount(() => {
    api.getSettings().then((s) => (settings = s));
    const sub = api.onSettings((s) => (settings = s));
    return () => sub.then((unlisten) => unlisten());
  });

  const themes: { value: ThemePref; label: string }[] = [
    { value: "system", label: "System" },
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
  ];

  function pickTheme(t: ThemePref) {
    if (settings) settings.theme = t;
    setTheme(t);
  }
</script>

<div class="page">
  <h1>Settings</h1>

  <section>
    <h2>Appearance</h2>
    <div class="row">
      <div>
        <div class="label">Theme</div>
        <div class="hint">System follows the Windows light/dark setting.</div>
      </div>
      <div class="segmented" role="radiogroup" aria-label="Theme">
        {#each themes as t}
          <button
            role="radio"
            aria-checked={settings?.theme === t.value}
            class:on={settings?.theme === t.value}
            onclick={() => pickTheme(t.value)}>{t.label}</button
          >
        {/each}
      </div>
    </div>
  </section>

  <section>
    <h2>Music</h2>
    <div class="row">
      <div class="grow">
        <div class="label">Music folder</div>
        <div class="hint path">{settings?.musicDir ?? "Not set"}</div>
      </div>
      <button class="btn" onclick={() => api.chooseMusicFolder()}>Change…</button>
    </div>
    <div class="row">
      <div>
        <div class="label">Library</div>
        <div class="hint">
          {#if status.scanning}Scanning…{:else}{status.count.toLocaleString()} tracks · unchanged files
            are not re-read{/if}
        </div>
      </div>
      <button class="btn" disabled={status.scanning || !settings?.musicDir} onclick={() => api.rescanLibrary()}
        >Rescan</button
      >
    </div>
  </section>
</div>

<style>
  .page {
    padding: 20px 24px;
    max-width: 640px;
    overflow-y: auto;
  }
  h1 {
    margin: 0 0 16px;
    font-size: 22px;
  }
  h2 {
    margin: 0 0 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--fg-faint);
  }
  section {
    margin-bottom: 22px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 14px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
  }
  .row + .row {
    border-top: 0;
  }
  .row:first-of-type {
    border-radius: 10px 10px 0 0;
  }
  .row:last-of-type {
    border-radius: 0 0 10px 10px;
  }
  .row:only-of-type {
    border-radius: 10px;
  }
  .grow {
    min-width: 0;
    flex: 1;
  }
  .label {
    font-weight: 600;
  }
  .hint {
    color: var(--fg-dim);
    margin-top: 2px;
  }
  .path {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .segmented {
    display: flex;
    padding: 2px;
    border-radius: 8px;
    background: var(--bg-active);
    flex: none;
  }
  .segmented button {
    padding: 5px 14px;
    border-radius: 6px;
    color: var(--fg-dim);
  }
  .segmented button.on {
    background: var(--bg-elevated);
    color: var(--fg);
    font-weight: 600;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
  }
  .btn {
    flex: none;
    padding: 6px 14px;
    border-radius: 7px;
    background: var(--bg-active);
    font-weight: 500;
  }
  .btn:hover:not(:disabled) {
    background: var(--track);
  }
  .btn:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
