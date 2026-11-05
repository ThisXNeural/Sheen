<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "../../shared/Icon.svelte";
  import { setTheme, type ThemePref } from "../../shared/theme";
  import * as api from "../api";
  import { store } from "../store.svelte";
  import ViewHeader from "../ViewHeader.svelte";

  let settings = $state<api.Settings | null>(null);

  onMount(() => {
    api.getSettings().then((s) => (settings = s));
    const sub = api.onSettings((s) => (settings = s));
    return () => sub.then((unlisten) => unlisten());
  });

  const themes: { value: ThemePref; label: string; icon: "monitor" | "sun" | "moon" }[] = [
    { value: "system", label: "System", icon: "monitor" },
    { value: "light", label: "Light", icon: "sun" },
    { value: "dark", label: "Dark", icon: "moon" },
  ];

  function pickTheme(t: ThemePref) {
    if (settings) settings.theme = t;
    setTheme(t);
  }

  const sizes = [
    { value: 0.85, label: "Small" },
    { value: 1, label: "Medium" },
    { value: 1.15, label: "Large" },
    { value: 1.3, label: "Extra large" },
  ];

  function pickSize(scale: number) {
    if (settings) settings.islandScale = scale;
    api.setIslandScale(scale);
  }

  function toggle(key: "clipboardPeek" | "showTransfers") {
    if (!settings) return;
    settings[key] = !settings[key];
    api.setIslandOption(key, settings[key]);
  }

  // ---- shortcut recorder -------------------------------------------------
  let recording = $state(false);
  let shortcutError = $state<string | null>(null);

  const MODIFIERS = new Set(["Control", "Alt", "Shift", "Meta", "OS"]);
  const CODE_NAMES: Record<string, string> = {
    Space: "Space", Enter: "Enter", Tab: "Tab", Backspace: "Backspace", Delete: "Delete",
    Insert: "Insert", Home: "Home", End: "End", PageUp: "PageUp", PageDown: "PageDown",
    ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right",
    ScrollLock: "ScrollLock", Pause: "Pause", PrintScreen: "PrintScreen", NumLock: "NumLock",
    CapsLock: "CapsLock", Semicolon: ";", Equal: "=", Comma: ",", Minus: "-", Period: ".",
    Slash: "/", Backquote: "`", BracketLeft: "[", Backslash: "\\", BracketRight: "]", Quote: "'",
    NumpadMultiply: "NumpadMultiply", NumpadAdd: "NumpadAdd", NumpadSubtract: "NumpadSubtract",
    NumpadDecimal: "NumpadDecimal", NumpadDivide: "NumpadDivide",
  };

  /** KeyboardEvent → "Ctrl+Alt+N" in the same names sheen-core parses. */
  function accelerator(e: KeyboardEvent): string | null {
    if (MODIFIERS.has(e.key)) return null;
    const code = e.code;
    const key =
      /^Key[A-Z]$/.test(code) ? code.slice(3)
      : /^Digit\d$/.test(code) ? code.slice(5)
      : /^Numpad\d$/.test(code) ? code
      : /^F\d{1,2}$/.test(code) ? code
      : CODE_NAMES[code];
    if (!key) return null;
    const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Win"].filter(Boolean);
    return [...mods, key].join("+");
  }

  // Sheen's own shortcut (e.g. Scroll Lock) would otherwise be swallowed by
  // the system before this page sees the key.
  function startRecording() {
    recording = true;
    shortcutError = null;
    api.suspendHotkeys(true);
  }

  function stopRecording() {
    recording = false;
    api.suspendHotkeys(false);
  }

  // Print Screen only produces a key-up event in browsers.
  function recordUp(e: KeyboardEvent) {
    if (recording && e.code === "PrintScreen") record(e);
  }

  async function record(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      stopRecording();
      return;
    }
    const plain = !e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey;
    if (plain && (e.key === "Backspace" || e.key === "Delete")) return save("");
    const acc = accelerator(e);
    if (acc) save(acc);
  }

  async function save(shortcut: string) {
    recording = false;
    try {
      const normalized = await api.setNoteShortcut(shortcut);
      shortcutError = null;
      if (settings) settings.noteShortcut = normalized;
    } catch (err) {
      shortcutError = String(err).replace(/^Error:\s*/, "");
    }
    // Re-register whatever is saved now (the new one, or the old on error).
    api.suspendHotkeys(false);
  }

  const pretty = (s: string) => s.replace("ScrollLock", "Scroll Lock").replaceAll("+", " + ");
</script>

<svelte:window onkeydown={record} onkeyup={recordUp} onblur={() => recording && stopRecording()} />

<ViewHeader title="Settings" />
<div class="page">
  <section>
    <h2>Appearance</h2>
    <div class="group">
      <div class="row">
        <div class="text">
          <div class="label">Theme</div>
          <div class="hint">System follows the Windows light/dark setting.</div>
        </div>
        <div class="segmented" role="radiogroup" aria-label="Theme">
          {#each themes as t}
            <button
              role="radio"
              aria-checked={settings?.theme === t.value}
              class:on={settings?.theme === t.value}
              onclick={() => pickTheme(t.value)}><Icon name={t.icon} size={14} />{t.label}</button
            >
          {/each}
        </div>
      </div>
    </div>
  </section>

  <section>
    <h2>Island</h2>
    <div class="group">
      <div class="row">
        <div class="text">
          <div class="label">Size</div>
          <div class="hint">How big the island is at the top of the screen.</div>
        </div>
        <div class="segmented" role="radiogroup" aria-label="Island size">
          {#each sizes as size}
            <button
              role="radio"
              aria-checked={settings?.islandScale === size.value}
              class:on={settings?.islandScale === size.value}
              onclick={() => pickSize(size.value)}>{size.label}</button
            >
          {/each}
        </div>
      </div>
      <div class="row">
        <div class="text">
          <div class="label">Downloads and file transfers</div>
          <div class="hint">Progress of browser downloads, IDM, and File Explorer copies (shown while the copy window is minimized or behind).</div>
        </div>
        <button
          class="switch"
          role="switch"
          aria-label="Downloads and file transfers"
          aria-checked={settings?.showTransfers ?? false}
          class:on={settings?.showTransfers}
          onclick={() => toggle("showTransfers")}><i></i></button
        >
      </div>
      <div class="row">
        <div class="text">
          <div class="label">Clipboard peek</div>
          <div class="hint">Briefly show what you copied. Password managers' private copies are never read.</div>
        </div>
        <button
          class="switch"
          role="switch"
          aria-label="Clipboard peek"
          aria-checked={settings?.clipboardPeek ?? false}
          class:on={settings?.clipboardPeek}
          onclick={() => toggle("clipboardPeek")}><i></i></button
        >
      </div>
    </div>
  </section>

  <section>
    <h2>Quick note</h2>
    <div class="group">
      <div class="row">
        <div class="text">
          <div class="label">Shortcut</div>
          <div class="hint" class:error={shortcutError}>
            {#if recording}Press a key combination — Esc to cancel, Backspace to turn off.
            {:else if shortcutError}{shortcutError}
            {:else}Opens the note from anywhere.{/if}
          </div>
        </div>
        <button class="keys" class:recording onclick={() => (recording ? stopRecording() : startRecording())}>
          <Icon name="keyboard" size={15} />
          {recording ? "Recording…" : settings?.noteShortcut ? pretty(settings.noteShortcut) : "Off"}
        </button>
      </div>
      <div class="row">
        <div class="text">
          <div class="label">Note</div>
          <div class="hint">Saved automatically as a text file in Sheen's data folder.</div>
        </div>
        <button class="btn" onclick={() => api.toggleNote()}><Icon name="note" size={15} /> Open</button>
      </div>
    </div>
  </section>

  <section>
    <h2>Music</h2>
    <div class="group">
      <div class="row">
        <div class="text">
          <div class="label">Music folder</div>
          <div class="hint path" dir="auto">{settings?.musicDir ?? "Not set"}</div>
        </div>
        <button class="btn" onclick={() => api.chooseMusicFolder()}><Icon name="folder" size={15} /> Change…</button>
      </div>
      <div class="row">
        <div class="text">
          <div class="label">Library</div>
          <div class="hint">
            {#if store.status.scanning}Scanning…{:else}{store.status.count.toLocaleString()} songs · only changed files are
              re-read{/if}
          </div>
        </div>
        <button class="btn" disabled={store.status.scanning || !settings?.musicDir} onclick={() => api.rescanLibrary()}
          >Rescan</button
        >
      </div>
    </div>
  </section>
</div>

<style>
  .page {
    flex: 1;
    overflow-y: auto;
    padding: 0 24px 24px;
  }
  section {
    max-width: 680px;
    margin-bottom: 22px;
  }
  h2 {
    margin: 0 0 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--fg-faint);
  }
  .group {
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    overflow: hidden;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 14px;
  }
  .row + .row {
    border-top: 1px solid var(--border);
  }
  .text {
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
  .hint.error {
    color: var(--danger);
  }
  .path {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .segmented {
    display: flex;
    flex: none;
    padding: 2px;
    border-radius: 8px;
    background: var(--bg-active);
  }
  .segmented button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border-radius: 6px;
    color: var(--fg-dim);
  }
  .segmented button.on {
    background: var(--bg-elevated);
    color: var(--fg);
    font-weight: 600;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
  }
  .btn,
  .keys {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: 7px;
    background: var(--bg-active);
    font-weight: 500;
    white-space: nowrap;
  }
  .btn:hover:not(:disabled),
  .keys:hover {
    background: var(--track);
  }
  .btn:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .keys {
    font-variant-numeric: tabular-nums;
  }
  .keys.recording {
    background: var(--accent);
    color: var(--accent-fg);
  }
  .switch {
    position: relative;
    flex: none;
    width: 40px;
    height: 24px;
    border-radius: 12px;
    background: var(--track);
    transition: background 0.2s;
  }
  .switch i {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
    transition: transform 0.2s;
  }
  .switch.on {
    background: var(--accent);
  }
  .switch.on i {
    transform: translateX(16px);
  }
</style>
