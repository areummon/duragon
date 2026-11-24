<script lang="ts">
  import { riddlerName } from '../state/riddler';
  import { app, updateMeta } from '../state/app.svelte';
  import { unfold } from '../motion/transitions';

  let draft = $state(app.note?.brief ?? '');
  let area = $state<HTMLTextAreaElement>();
  const changed = $derived(draft.trim() !== (app.note?.brief ?? ''));

  $effect(() => {
    area?.focus();
    area?.setSelectionRange(draft.length, draft.length);
  });

  function cancel() {
    app.tellOpen = false;
  }
  async function save() {
    app.tellOpen = false;
    await updateMeta({ brief: draft.trim() });
  }
  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      cancel();
    } else if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      void save();
    }
  }
</script>

<div class="tell" transition:unfold={{ duration: 260 }}>
  <div class="card">
    <label for="tell-brief">What's this note for, or how should {riddlerName()} be here? It keeps this in mind.</label>
    <textarea
      id="tell-brief"
      bind:this={area}
      bind:value={draft}
      {onkeydown}
      rows="2"
      placeholder="Prep for a system design interview. Be tough on hand-waving."
    ></textarea>
    <div class="actions">
      <button class="cancel" onclick={cancel}>Cancel</button>
      <button class="save" disabled={!changed} onclick={save}>Save</button>
    </div>
  </div>
</div>

<style>
  .tell { padding-top: 12px; padding-bottom: 12px; }
  .card {
    padding: 14px 16px 12px;
    border: 1px solid var(--rule);
    border-radius: 14px;
  }
  label { display: block; font-size: 12px; line-height: 1.4; color: var(--faint); }
  textarea {
    display: block;
    width: 100%;
    margin-top: 6px;
    padding: 0;
    border: 0;
    background: transparent;
    resize: none;
    outline: none;
    font-family: var(--font-voice);
    font-size: 17px;
    line-height: 1.5;
    color: var(--ink);
    field-sizing: content;
    min-height: 3em;
  }
  textarea::placeholder { color: var(--faint); opacity: 0.8; }
  .actions { display: flex; justify-content: flex-end; align-items: center; gap: 8px; }
  .cancel { padding: 6px 10px; font-size: 13px; color: var(--faint); transition: color var(--dur-ui) var(--ease-ui); }
  .cancel:hover { color: var(--ink); }
  .save {
    height: 30px;
    padding: 0 14px;
    border-radius: 999px;
    background: var(--ink);
    color: var(--paper);
    font-size: 13px;
    transition: opacity var(--dur-ui) var(--ease-ui), transform 160ms var(--ease-step);
  }
  .save:disabled { opacity: 0.3; cursor: default; }
  /* Gives a little under the press. */
  .save:active:not(:disabled) { transform: scale(0.97); }
  @media (prefers-reduced-motion: reduce) { .save:active:not(:disabled) { transform: none; } }
</style>
