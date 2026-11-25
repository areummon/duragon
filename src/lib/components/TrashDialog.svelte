<script lang="ts">
  import X from 'lucide-svelte/icons/x';
  import { tick } from 'svelte';
  import type { TrashedNote } from '../api/types';
  import { closeOnEscape } from '../escape';
  import { focusLost, takeFocus } from '../focus';
  import { pop, unfold, veil } from '../motion/transitions';
  import { app, restoreNote } from '../state/app.svelte';
  import { deleteForever, forgetTrash, loadTrash, trash } from '../state/trash.svelte';

  let card = $state<HTMLElement>();
  /** What waits for a yes, as it was when asked: a row's note, or every note in the list. */
  let asking = $state<{ kind: 'row' | 'all'; ids: string[] } | null>(null);
  let busy = $state(false);

  const close = () => (app.trashOpen = false);
  const askingFor = (n: TrashedNote) => asking?.kind === 'row' && asking.ids[0] === n.id;

  // Before the page updates, so an old list is never drawn.
  $effect.pre(() => {
    if (!app.trashOpen) return;
    forgetTrash();
    void loadTrash();
    return () => (asking = null);
  });
  $effect(() => {
    if (app.trashOpen) return closeOnEscape(close);
  });
  $effect(() => {
    if (asking) return closeOnEscape(() => (asking = null));
  });

  /** Focus that went with a row goes to the row that took its place. */
  async function keepFocus(row: number) {
    await tick();
    if (!card || !(focusLost() || document.activeElement === card)) return;
    const rows = card.querySelectorAll<HTMLElement>('li:not([inert]) .restore');
    (rows[Math.min(row, rows.length - 1)] ?? card).focus({ preventScroll: true });
  }

  async function act(work: () => Promise<unknown>, row = 0) {
    if (busy) return;
    busy = true;
    try {
      await work();
      asking = null;
    } finally {
      busy = false;
    }
    if (app.trashOpen) await keepFocus(row);
  }

  async function ask(what: typeof asking) {
    asking = what;
    await tick();
    card?.querySelector<HTMLElement>('.keep')?.focus({ preventScroll: true });
  }

  async function keep(row = 0) {
    asking = null;
    await keepFocus(row);
  }

  function when(ms: number) {
    const d = new Date(ms);
    const now = new Date();
    if (d.toDateString() === now.toDateString()) {
      return `Deleted today at ${d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })}`;
    }
    const year = d.getFullYear() === now.getFullYear() ? undefined : 'numeric';
    return `Deleted on ${d.toLocaleDateString(undefined, { day: 'numeric', month: 'long', year })}`;
  }
</script>

<svelte:window onfocus={() => { if (app.trashOpen) void loadTrash(); }} />

{#if app.trashOpen}
  <div class="scrim" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()} transition:veil|global>
    <div class="card" bind:this={card} role="dialog" aria-modal="true" aria-labelledby="trash-title" tabindex="-1" use:takeFocus in:pop|global out:pop={{ duration: 140 }}>
      <button class="x" aria-label="Close" onclick={close}><X size={17} strokeWidth={1.5} /></button>
      <h2 id="trash-title">Trash</h2>
      <p class="sub">Deleted notes stay here until you restore them or delete them for good.</p>

      {#if trash.loaded && trash.notes.length === 0}
        <p class="empty">The trash is empty.</p>
      {/if}
      <ul>
        {#each trash.notes as n, i (n.id)}
          <li out:unfold={{ duration: 200 }}>
            <div class="row">
              <div class="what">
                <div class="title" id="trash-{n.id}" title={n.title || 'Untitled'}>{n.title || 'Untitled'}</div>
                <div class="when">{askingFor(n) ? 'Delete this note for good?' : when(n.deletedAt)}</div>
              </div>
              {#if askingFor(n)}
                <button class="quiet danger" aria-describedby="trash-{n.id}" disabled={busy} onclick={() => act(() => deleteForever([n.id]), i)}>Delete</button>
                <button class="quiet keep" onclick={() => keep(i)}>Keep it</button>
              {:else}
                <button class="quiet restore" aria-describedby="trash-{n.id}" disabled={busy} onclick={() => act(() => restoreNote(n.id), i)}>Restore</button>
                <button class="quiet" aria-describedby="trash-{n.id}" disabled={busy} onclick={() => ask({ kind: 'row', ids: [n.id] })}>Delete for good</button>
              {/if}
            </div>
          </li>
        {/each}
      </ul>

      {#if trash.unreadable > 0}
        <p class="unreadable">
          {trash.unreadable === 1
            ? 'One file here can’t be read by this Duragon, so it was left as it is.'
            : `${trash.unreadable} files here can’t be read by this Duragon, so they were left as they are.`}
        </p>
      {/if}
      {#if trash.notes.length > 0}
        <div class="foot">
          {#if asking?.kind === 'all'}
            {@const ids = asking.ids}
            <span class="grow">{ids.length === 1 ? 'Delete this note for good?' : `Delete ${ids.length} notes for good?`}</span>
            <button class="quiet danger" disabled={busy} onclick={() => act(() => deleteForever(ids))}>Delete</button>
            <button class="quiet keep" onclick={() => keep()}>{ids.length === 1 ? 'Keep it' : 'Keep them'}</button>
          {:else}
            <span class="grow"></span>
            <button class="quiet" disabled={busy} onclick={() => ask({ kind: 'all', ids: trash.notes.map((n) => n.id) })}>Empty trash</button>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .scrim { position: fixed; inset: 0; z-index: 55; display: grid; grid-template-columns: minmax(0, 1fr); place-items: center; padding: 24px; background: color-mix(in srgb, var(--paper) 55%, transparent); }
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    width: min(560px, 100%);
    max-height: calc(100vh - 48px);
    padding: 26px 0 0;
    border-radius: 14px;
    background: var(--paper);
    box-shadow: var(--lift);
    font-size: 14px;
    line-height: 1.4;
    outline: none;
  }
  .x { position: absolute; top: 16px; right: 16px; display: grid; place-items: center; width: 28px; height: 28px; color: var(--graphite); }
  .x:hover { color: var(--ink); }
  h2 { padding: 0 32px; font-size: 19px; line-height: 1.3; font-weight: 500; }
  .sub { margin-top: 4px; padding: 0 32px 18px; font-size: 13.5px; line-height: 1.45; color: var(--faint); }
  .empty { padding: 14px 32px 30px; color: var(--graphite); }
  ul { flex: 1; min-height: 0; margin: 0; padding: 0 22px; overflow-y: auto; overscroll-behavior: contain; list-style: none; scrollbar-width: thin; scrollbar-color: var(--rule) transparent; }
  .row { display: flex; align-items: center; gap: 4px; padding: 10px 0 10px 10px; border-top: 1px solid var(--rule); }
  .what { flex: 1; min-width: 0; margin-right: 10px; }
  .title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 14.5px; }
  .when { margin-top: 1px; font-size: 13px; color: var(--faint); }
  .unreadable { padding: 12px 32px; border-top: 1px solid var(--rule); font-size: 13px; line-height: 1.45; color: var(--faint); }
  .foot { display: flex; align-items: center; gap: 4px; padding: 10px 22px 12px 32px; border-top: 1px solid var(--rule); }
  .grow { flex: 1; font-size: 13.5px; color: var(--graphite); }
  .quiet { flex: none; height: 30px; padding: 0 10px; border-radius: 8px; font-size: 13.5px; color: var(--graphite); transition: color 150ms var(--ease-ui), background-color 150ms var(--ease-ui); }
  .quiet:hover { color: var(--ink); background: var(--riddle-wash); }
  .quiet.danger, .quiet.danger:hover { color: var(--text-red); }
</style>
