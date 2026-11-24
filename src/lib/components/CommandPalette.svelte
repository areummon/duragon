<script lang="ts">
  import FileText from 'lucide-svelte/icons/file-text';
  import Plus from 'lucide-svelte/icons/plus';
  import { getBackend } from '../api/backend';
  import type { NoteMeta } from '../api/types';
  import { closeOnEscape } from '../escape';
  import { app, newNote, openNote } from '../state/app.svelte';

  let query = $state('');
  let results = $state<NoteMeta[]>([]);
  let index = $state(0);
  let input = $state<HTMLInputElement>();
  let seq = 0;

  const typed = $derived(query.trim());
  const canCreate = $derived(typed !== '' && !results.some((r) => r.title.toLowerCase() === typed.toLowerCase()));
  const count = $derived(results.length + (canCreate ? 1 : 0));

  $effect(() => {
    const q = query;
    const mine = ++seq;
    void getBackend()
      .then((b) => b.searchNotes(q))
      .then((r) => {
        if (mine !== seq) return;
        results = r.slice(0, 8);
        index = 0;
      });
  });

  function close() {
    app.paletteOpen = false;
  }

  // Before the field takes focus, so Escape hands focus back to what had it.
  $effect(() => closeOnEscape(close));
  $effect(() => {
    input?.focus();
  });

  async function pick(i: number) {
    close();
    if (i < results.length) await openNote(results[i].id);
    else if (canCreate) await newNote(typed);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' && count) {
      e.preventDefault();
      index = (index + 1) % count;
    } else if (e.key === 'ArrowUp' && count) {
      e.preventDefault();
      index = (index - 1 + count) % count;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (count) void pick(index);
    }
  }
</script>

<div class="scrim" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()}>
  <div class="box" role="dialog" aria-modal="true" aria-label="Find a note">
    <input
      bind:this={input}
      bind:value={query}
      {onkeydown}
      role="combobox"
      aria-expanded="true"
      aria-controls="palette-list"
      aria-activedescendant={count ? `palette-${index}` : undefined}
      aria-label="Find a note, or name a new one"
      placeholder="Find a note, or name a new one"
      spellcheck="false"
      autocomplete="off"
    />
    {#if count}
      <div class="list" id="palette-list" role="listbox" aria-label="Notes">
        {#each results as n, i (n.id)}
          <div id="palette-{i}" class="item" class:active={i === index} role="option" aria-selected={i === index} tabindex="-1"
            onpointerdown={(e) => e.preventDefault()} onclick={() => pick(i)} onmousemove={() => (index = i)} onkeydown={() => {}}>
            <FileText size={15} strokeWidth={1.5} /><span>{n.title || 'Untitled'}</span>
          </div>
        {/each}
        {#if canCreate}
          {@const i = results.length}
          <div id="palette-{i}" class="item" class:active={i === index} role="option" aria-selected={i === index} tabindex="-1"
            onpointerdown={(e) => e.preventDefault()} onclick={() => pick(i)} onmousemove={() => (index = i)} onkeydown={() => {}}>
            <Plus size={15} strokeWidth={1.5} /><span>New note: {typed}</span>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 18vh;
    background: color-mix(in srgb, var(--paper) 55%, transparent);
  }
  .box {
    width: min(560px, calc(100vw - 32px));
    background: var(--paper);
    border: 1px solid var(--rule);
    border-radius: 14px;
    box-shadow: var(--lift);
    overflow: hidden;
  }
  input {
    width: 100%;
    height: 52px;
    padding: 0 18px;
    border: 0;
    background: transparent;
    font: inherit;
    font-size: 16px;
    color: var(--ink);
    outline: none;
  }
  input::placeholder { color: var(--faint); }
  .list { padding: 4px; border-top: 1px solid var(--rule); }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 12px;
    border-radius: 8px;
    font-size: 14px;
    color: var(--ink);
    cursor: default;
  }
  .item :global(svg) { color: var(--faint); flex: none; }
  .item span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .item.active { background: var(--riddle-wash); }
</style>
