<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick } from "svelte";
  import { inTauri } from "../shared/env";
  import Icon from "../shared/Icon.svelte";
  import { initTheme } from "../shared/theme";

  let text = $state("");
  let open = $state(!inTauri);
  let pinned = $state(false);
  let saved = $state(true);
  let copied = $state(false);
  let area = $state<HTMLTextAreaElement>();

  const call = (cmd: string, args?: Record<string, unknown>) =>
    inTauri ? invoke(cmd, args) : Promise.resolve(undefined);

  // Autosave shortly after typing stops, and always before hiding.
  let timer: ReturnType<typeof setTimeout> | undefined;
  function changed() {
    saved = false;
    clearTimeout(timer);
    timer = setTimeout(save, 350);
  }
  async function save() {
    clearTimeout(timer);
    if (saved) return;
    await call("note_save", { text });
    saved = true;
  }

  async function show() {
    open = true;
    await tick();
    area?.focus();
  }

  async function hide() {
    if (!open) return;
    await save();
    open = false;
  }

  // The window is hidden only after the closing animation.
  function closed() {
    if (!open) call("note_hidden");
  }

  async function copyAll() {
    await navigator.clipboard.writeText(text);
    copied = true;
    setTimeout(() => (copied = false), 1200);
  }

  function togglePin() {
    pinned = !pinned;
    call("note_set_pinned", { pinned });
  }

  onMount(() => {
    initTheme();
    if (inTauri) {
      invoke<string>("note_load").then((t) => (text = t));
      // The window is only ever created right before it is shown, and the
      // "shown" event from that first open fires before this page has
      // loaded. Open now, or the card stays invisible.
      show();
      const subs = [listen("sheen://note-shown", show), listen("sheen://note-hide", hide)];
      return () => subs.forEach((s) => s.then((u) => u()));
    } else {
      text = new URLSearchParams(location.search).get("text") ?? "Buy milk\nCall Sara back about the invoice\n— draft ideas for the weekend";
    }
  });

  const words = $derived(text.trim() ? text.trim().split(/\s+/).length : 0);
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && hide()} />

<div class="wrap">
  <section class="card" class:open ontransitionend={closed} aria-label="Quick note">
    <header>
      <span class="title"><Icon name="note" size={15} /> Note</span>
      <span class="status" class:dirty={!saved}>{saved ? "Saved" : "Saving…"}</span>
      <button class:on={pinned} aria-label={pinned ? "Unpin" : "Keep open"} aria-pressed={pinned} onclick={togglePin}>
        <Icon name={pinned ? "pin" : "pinOff"} size={14} />
      </button>
      <button aria-label="Close" onclick={hide}><Icon name="x" size={15} /></button>
    </header>
    <textarea
      bind:this={area}
      bind:value={text}
      oninput={changed}
      onblur={save}
      placeholder="Jot something down…"
      spellcheck="true"
      dir="auto"
    ></textarea>
    <footer>
      <span>{words} {words === 1 ? "word" : "words"}</span>
      <button onclick={copyAll} disabled={!text}>
        <Icon name={copied ? "check" : "copy"} size={13} />
        {copied ? "Copied" : "Copy"}
      </button>
    </footer>
  </section>
</div>

<style>
  .wrap {
    height: 100%;
    padding: 6px 14px 20px;
    box-sizing: border-box;
  }
  .card {
    height: 100%;
    display: flex;
    flex-direction: column;
    border-radius: 18px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    box-shadow:
      0 12px 32px rgba(0, 0, 0, 0.22),
      0 2px 6px rgba(0, 0, 0, 0.12);
    overflow: hidden;
    box-sizing: border-box;
    /* Drops down from the island. */
    transform-origin: 50% 0;
    transform: translateY(-14px) scale(0.94);
    opacity: 0;
    transition:
      transform 0.22s cubic-bezier(0.2, 0.9, 0.3, 1.2),
      opacity 0.16s ease;
  }
  .card.open {
    transform: none;
    opacity: 1;
  }
  @media (prefers-reduced-motion: reduce) {
    .card {
      transition: opacity 0.12s;
      transform: none;
    }
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 10px 10px 4px 14px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-weight: 700;
  }
  .status {
    flex: 1;
    font-size: 11px;
    color: var(--fg-faint);
  }
  .status.dirty {
    color: var(--fg-dim);
  }
  header button {
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 7px;
    color: var(--fg-dim);
  }
  header button:hover {
    background: var(--bg-hover);
    color: var(--fg);
  }
  header button.on {
    color: var(--accent);
  }
  textarea {
    flex: 1;
    min-height: 0;
    margin: 0;
    padding: 6px 14px;
    border: 0;
    outline: none;
    resize: none;
    background: transparent;
    color: var(--fg);
    font: inherit;
    font-size: 14px;
    line-height: 1.55;
  }
  textarea::placeholder {
    color: var(--fg-faint);
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px 8px 14px;
    font-size: 11px;
    color: var(--fg-faint);
    border-top: 1px solid var(--border);
  }
  footer button {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 3px 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--fg-dim);
  }
  footer button:hover:not(:disabled) {
    background: var(--bg-hover);
  }
  footer button:disabled {
    opacity: 0.4;
  }
</style>
