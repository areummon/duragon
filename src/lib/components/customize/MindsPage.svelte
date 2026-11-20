<script lang="ts">
  import { app } from '../../state/app.svelte';
  import { notes } from '../../state/notes.svelte';
  import Blot from '../Blot.svelte';

  const builtin = $derived(notes.riddlers.filter((r) => r.builtin));
  const yours = $derived(notes.riddlers.filter((r) => !r.builtin));
</script>

<div class="head">
  <p class="intro">Minds that read your notes, each with its own way of asking. Pick one for any note from the line above its title.</p>
  <button class="new" onclick={() => (app.mindEditor = { id: null })}>New Mind</button>
</div>

<div class="list">
  {#each builtin as r (r.id)}
    <div class="mind">
      <span class="dot"><Blot size={15} seed="duragon" /></span>
      <div><div class="name">{r.name}</div><div class="desc">{r.description}</div></div>
    </div>
  {/each}
</div>

<h4>Yours</h4>
{#if yours.length === 0}
  <p class="empty">None yet. Start from a staff engineer, an editor, a tutor, or write your own.</p>
{:else}
  <div class="list">
    {#each yours as r (r.id)}
      <div class="mind">
        <span class="dot"><Blot size={15} seed={r.id} /></span>
        <div class="grow"><div class="name">{r.name}</div><div class="desc">{r.description}</div></div>
        <button class="edit" aria-label="Edit {r.name}" onclick={() => (app.mindEditor = { id: r.id })}>Edit</button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .head { display: flex; align-items: flex-start; justify-content: space-between; gap: 24px; }
  .intro { max-width: 410px; font-size: 14px; line-height: 1.5; color: var(--graphite); }
  .new { flex: none; height: 36px; padding: 0 16px; border-radius: 999px; background: var(--ink); color: var(--paper); font-size: 13.5px; transition: opacity 150ms var(--ease-ui); }
  .new:hover { opacity: 0.85; }
  .list { margin-top: 24px; display: flex; flex-direction: column; gap: 18px; }
  .mind { display: flex; align-items: flex-start; gap: 12px; }
  .grow { flex: 1; }
  .dot { display: flex; margin-top: 5px; color: var(--riddle); }
  .name { font-size: 14.5px; line-height: 1.4; }
  .desc { margin-top: 2px; font-size: 13.5px; line-height: 1.45; color: var(--graphite); }
  h4 { margin: 34px 0 0; font-size: 13px; font-weight: 400; color: var(--faint); }
  .empty { margin-top: 16px; font-size: 14px; line-height: 1.5; color: var(--graphite); }
  .edit { height: 30px; padding: 0 12px; margin-top: 3px; border-radius: 8px; font-size: 13.5px; color: var(--graphite); transition: color 150ms var(--ease-ui), background-color 150ms var(--ease-ui); }
  .edit:hover { color: var(--ink); background: var(--riddle-wash); }
</style>
