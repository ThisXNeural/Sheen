<script lang="ts">
  import { prompt } from "./menu.svelte";

  let input = $state<HTMLInputElement>();
  $effect(() => {
    if (prompt.open) queueMicrotask(() => input?.select());
  });
</script>

{#if prompt.open}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="scrim" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.target === e.currentTarget && prompt.done(null)}>
    <form
      class="dialog"
      aria-label={prompt.title}
      onsubmit={(e) => {
        e.preventDefault();
        prompt.done(prompt.value);
      }}
    >
      <h2>{prompt.title}</h2>
      <input
        bind:this={input}
        bind:value={prompt.value}
        dir="auto"
        placeholder="Name"
        maxlength="120"
        onkeydown={(e) => e.key === "Escape" && prompt.done(null)}
      />
      <div class="buttons">
        <button type="button" class="ghost" onclick={() => prompt.done(null)}>Cancel</button>
        <button type="submit" class="primary">{prompt.confirm}</button>
      </div>
    </form>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.3);
    padding: 16px;
  }
  .dialog {
    width: min(360px, 100%);
    padding: 18px;
    border-radius: 14px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    box-shadow: var(--shadow);
    box-sizing: border-box;
  }
  h2 {
    margin: 0 0 12px;
    font-size: 15px;
  }
  input {
    width: 100%;
    box-sizing: border-box;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--bg);
    outline: none;
  }
  input:focus {
    border-color: var(--accent);
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 14px;
  }
  .buttons button {
    padding: 7px 14px;
    border-radius: 8px;
    font-weight: 600;
  }
  .ghost {
    background: var(--bg-active);
  }
  .primary {
    background: var(--accent);
    color: var(--accent-fg);
  }
</style>
