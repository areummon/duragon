<script lang="ts">
  import X from 'lucide-svelte/icons/x';
  import { getBackend } from '../api/backend';
  import type { NoteRevision } from '../api/types';
  import { closeOnEscape } from '../escape';
  import { takeFocus } from '../focus';
  import { pop, veil } from '../motion/transitions';
  import { app, restoreNoteRevision } from '../state/app.svelte';

  let card = $state<HTMLElement>();
  let revisions = $state<NoteRevision[]>([]);
  let selectedId = $state<string | null>(null);
  let loaded = $state(false);
  let restoring = $state(false);

  const close = () => (app.historyOpen = false);
  const selected = $derived(revisions.find((r) => r.id === selectedId) ?? revisions[0] ?? null);
  const previewBody = $derived.by(() => {
    if (!selected) return '';
    const title = selected.title.trim();
    const plain = selected.plain;
    if (title && (plain === title || plain.startsWith(`${title}\n`))) {
      return plain.slice(title.length).replace(/^\n+/, '');
    }
    return plain;
  });

  function onListKeydown(e: KeyboardEvent) {
    if (revisions.length < 2) return;
    const idx = revisions.findIndex((r) => r.id === selected?.id);
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedId = revisions[Math.min(revisions.length - 1, Math.max(0, idx) + 1)].id;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedId = revisions[Math.max(0, idx - 1)].id;
    }
  }

  async function load(noteId: string) {
    loaded = false;
    revisions = [];
    selectedId = null;
    try {
      const list = await (await getBackend()).listRevisions(noteId);
      if (app.note?.id !== noteId) return;
      revisions = list;
      selectedId = list[0]?.id ?? null;
    } finally {
      loaded = true;
    }
  }

  $effect.pre(() => {
    if (!app.historyOpen || !app.note) return;
    void load(app.note.id);
  });

  $effect(() => {
    if (app.historyOpen) return closeOnEscape(close);
  });

  async function restore() {
    if (!selected || restoring) return;
    restoring = true;
    try {
      await restoreNoteRevision(selected.id);
    } finally {
      restoring = false;
    }
  }

  function formatWhen(ms: number): string {
    const d = new Date(ms);
    const now = new Date();
    const time = d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
    if (d.toDateString() === now.toDateString()) return `Today, ${time}`;
    const yesterday = new Date(now);
    yesterday.setDate(now.getDate() - 1);
    if (d.toDateString() === yesterday.toDateString()) return `Yesterday, ${time}`;
    const year = d.getFullYear() === now.getFullYear() ? undefined : 'numeric';
    const day = d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year });
    return `${day}, ${time}`;
  }
</script>

{#if app.historyOpen}
  <div class="scrim" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()} transition:veil|global>
    <div
      class="card"
      class:is-empty={loaded && revisions.length === 0}
      bind:this={card}
      role="dialog"
      aria-modal="true"
      aria-label="Note history"
      tabindex="-1"
      use:takeFocus
      in:pop|global
      out:pop={{ duration: 140 }}
    >
      {#if loaded && revisions.length === 0}
        <button class="x" aria-label="Close" onclick={close}><X size={17} strokeWidth={1.5} /></button>
        <div class="empty-wrap">
          <h2>History</h2>
          <p class="empty">Snapshots are saved automatically as you write.</p>
        </div>
      {:else}
        <aside aria-label="Saved snapshots">
          <div class="aside-head">History</div>
          <div class="list" role="listbox" aria-label="Revisions" tabindex="0" onkeydown={onListKeydown}>
            {#each revisions as rev (rev.id)}
              {@const active = selected?.id === rev.id}
              <button
                class="rev"
                class:active
                role="option"
                aria-selected={active}
                onclick={() => (selectedId = rev.id)}
              >
                <span class="rev-time">{formatWhen(rev.createdAt)}</span>
                <span class="rev-title">{rev.title || 'Untitled'}</span>
              </button>
            {/each}
          </div>
        </aside>

        <div class="main">
          <button class="x" aria-label="Close" onclick={close}><X size={17} strokeWidth={1.5} /></button>
          {#if selected}
            <div class="head">
              <div class="head-title">{selected.title || 'Untitled'}</div>
              <div class="head-time">{formatWhen(selected.createdAt)}</div>
            </div>
            <div class="preview">
              {#if previewBody}
                <div class="plain">{previewBody}</div>
              {:else}
                <p class="empty-body">Empty note.</p>
              {/if}
            </div>
            <div class="foot">
              <span class="note">Your current draft is saved first.</span>
              <button class="restore" disabled={restoring} onclick={restore}>
                {restoring ? 'Restoring…' : 'Restore this draft'}
              </button>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    place-items: center;
    padding: 24px;
    background: var(--veil);
  }
  .card {
    position: relative;
    display: flex;
    width: min(760px, 100%);
    height: min(560px, calc(100vh - 48px));
    overflow: hidden;
    border-radius: 14px;
    background: var(--paper);
    box-shadow: var(--lift);
    font-size: 14px;
    line-height: 1.4;
    outline: none;
  }
  .card.is-empty {
    display: block;
    width: min(460px, 100%);
    height: auto;
  }
  .empty-wrap {
    padding: 26px 32px 30px;
  }
  h2 {
    margin: 0;
    font-size: 19px;
    line-height: 1.3;
    font-weight: 500;
    letter-spacing: -0.01em;
  }
  .empty {
    margin-top: 8px;
    font-size: 14px;
    line-height: 1.5;
    color: var(--graphite);
  }
  aside {
    flex: none;
    width: 224px;
    display: flex;
    flex-direction: column;
    background: var(--shelf);
    border-right: 1px solid var(--rule);
  }
  .aside-head {
    padding: 22px 20px 12px;
    font-size: 15px;
    line-height: 1.3;
    font-weight: 500;
    color: var(--ink);
  }
  .list {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 10px 14px;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--rule) transparent;
  }
  .rev {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    padding: 8px 10px;
    border-radius: 8px;
    text-align: left;
    color: var(--graphite);
    transition: color 150ms var(--ease-ui), background-color 150ms var(--ease-ui);
  }
  @media (hover: hover) and (pointer: fine) {
    .rev:hover {
      color: var(--ink);
      background: var(--riddle-wash);
    }
  }
  .rev.active {
    color: var(--ink);
    background: var(--paper);
    box-shadow: inset 0 0 0 1px var(--rule);
  }
  .rev-time {
    font-size: 13.5px;
    line-height: 1.3;
    font-weight: 500;
  }
  .rev-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12.5px;
    line-height: 1.3;
    color: var(--faint);
  }
  .main {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .x {
    position: absolute;
    top: 18px;
    right: 18px;
    z-index: 2;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    color: var(--faint);
    transition: color 150ms var(--ease-ui), background-color 150ms var(--ease-ui);
  }
  @media (hover: hover) and (pointer: fine) {
    .x:hover {
      color: var(--ink);
      background: var(--shelf);
    }
  }
  .head {
    padding: 22px 56px 14px 28px;
    border-bottom: 1px solid var(--rule);
  }
  .head-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 16px;
    line-height: 1.3;
    font-weight: 500;
    color: var(--ink);
  }
  .head-time {
    margin-top: 2px;
    font-size: 12.5px;
    line-height: 1.3;
    color: var(--faint);
  }
  .preview {
    flex: 1;
    min-height: 0;
    padding: 20px 28px;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--rule) transparent;
  }
  .plain {
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 14.5px;
    line-height: 1.65;
    color: var(--ink);
  }
  .empty-body {
    margin: 0;
    font-size: 14px;
    line-height: 1.5;
    color: var(--faint);
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 24px 14px 28px;
    border-top: 1px solid var(--rule);
  }
  .note {
    font-size: 12.5px;
    line-height: 1.3;
    color: var(--faint);
  }
  .restore {
    flex: none;
    height: 32px;
    padding: 0 14px;
    border-radius: 8px;
    background: var(--ink);
    color: var(--paper);
    font-size: 13.5px;
    font-weight: 500;
    line-height: 1;
    text-box: trim-both cap alphabetic;
    transition: background-color 150ms var(--ease-ui), transform 140ms var(--ease-step);
  }
  .restore:hover:not(:disabled) {
    background: color-mix(in srgb, var(--ink) 86%, var(--paper));
  }
  .restore:active:not(:disabled) {
    transform: scale(0.98);
  }
  .restore:disabled {
    opacity: 0.5;
  }
  @media (prefers-reduced-motion: reduce) {
    .restore:active:not(:disabled) {
      transform: none;
    }
  }
  @media (max-width: 620px) {
    aside {
      width: 168px;
    }
    .head {
      padding: 18px 52px 12px 20px;
    }
    .preview {
      padding: 16px 20px;
    }
    .foot {
      padding: 12px 16px 14px 20px;
    }
  }
</style>
