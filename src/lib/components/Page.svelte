<script lang="ts">
  import { untrack } from 'svelte';
  import { getBackend } from '../api/backend';
  import type { NoteMeta } from '../api/types';
  import { closeOnEscape } from '../escape';
  import EditorView, { type EditorApi } from '../editor/Editor.svelte';
  import { app, openNote } from '../state/app.svelte';
  import { notes } from '../state/notes.svelte';
  import { riddlerName } from '../state/riddler';
  import {
    PAGE_WIDTH_DEFAULT, PAGE_WIDTH_MAX, PAGE_WIDTH_MIN, updateSettings,
  } from '../state/settings.svelte';
  import { attachRiddle, type Riddle } from '../editor/riddle.svelte';
  import { chat } from '../state/chat.svelte';
  import { loadThreads, unloadThreads } from '../state/threads.svelte';
  import Pencil from 'lucide-svelte/icons/pencil';
  import { interruptSpeech } from '../state/voice.svelte';
  import { layout, pageWidth } from '../state/layout.svelte';
  import Margin from './Margin.svelte';
  import ReadingPencil from './ReadingPencil.svelte';
  import SelectionMenu from './SelectionMenu.svelte';
  import NoteHeaderRow from './NoteHeaderRow.svelte';
  import TellCard from './TellCard.svelte';

  let api = $state.raw<EditorApi | null>(null);
  let page = $state<HTMLElement>();
  let riddle = $state.raw<Riddle | null>(null);
  let dragSide = $state<-1 | 1 | null>(null);
  let backlinks = $state<NoteMeta[]>([]);
  const noteId = $derived(app.note?.id);

  $effect(() => {
    const id = noteId;
    // Re-read backlinks whenever the open note changes or any note is saved/updated.
    void notes.list;
    if (!id) {
      backlinks = [];
      return;
    }
    let cancelled = false;
    void getBackend()
      .then((be) => be.listBacklinks(id))
      .then((list) => {
        if (!cancelled) backlinks = list;
      })
      .catch(() => {
        if (!cancelled) backlinks = [];
      });
    return () => {
      cancelled = true;
    };
  });

  $effect(() => {
    const id = noteId;
    if (!id) return;
    void loadThreads(id);
    return unloadThreads;
  });

  $effect(() => {
    if (!api || !page) return;
    const id = untrack(() => noteId);
    if (!id) return;
    const r = attachRiddle(api.editor, page, id);
    riddle = r;
    return () => {
      r.detach();
      riddle = null;
    };
  });

  $effect(() => {
    if (app.tellOpen) return closeOnEscape(() => (app.tellOpen = false), () => page?.querySelector<HTMLElement>('[data-tell]'));
  });

  $effect(() => {
    if (!app.focusTitle || !api) return;
    app.focusTitle = false;
    const { editor } = api;
    const select = () => {
      const title = editor.state.doc.firstChild!;
      editor.chain().focus().setTextSelection({ from: 1, to: 1 + title.content.size }).run();
    };
    // A note that just opened focuses itself once its editor starts, which would move the caret off the title.
    if (editor.isInitialized) select();
    else editor.once('create', select);
  });

  /** A press on the blank page below the note goes on writing in its last line. */
  function writeBelow(e: MouseEvent) {
    if (!api || !page || e.button !== 0) return;
    const { view, commands } = api.editor;
    const blank = e.target === page || e.target === page.parentElement || e.target === view.dom.parentElement;
    const { left, right, bottom } = view.dom.getBoundingClientRect();
    if (!blank || e.clientY < bottom || e.clientX < left || e.clientX > right) return;
    e.preventDefault();
    commands.focus('end', { scrollIntoView: false });
  }

  function drag(side: -1 | 1) {
    return (e: PointerEvent) => {
      if (e.button !== 0) return;
      const el = e.currentTarget as HTMLElement;
      el.setPointerCapture(e.pointerId);
      const startX = e.clientX;
      const start = page!.getBoundingClientRect().width;
      let width = Math.round(start);
      dragSide = side;
      layout.dragWidth = width;
      const move = (m: PointerEvent) => {
        width = Math.round(Math.min(PAGE_WIDTH_MAX, Math.max(PAGE_WIDTH_MIN, start + side * 2 * (m.clientX - startX))));
        layout.dragWidth = width;
      };
      const up = (u: PointerEvent) => {
        el.removeEventListener('pointermove', move);
        el.removeEventListener('pointerup', up);
        el.removeEventListener('pointercancel', up);
        // A window narrower than the page shows it narrower, and a press alone doesn't keep that.
        if (u.clientX !== startX) void updateSettings({ pageWidth: width === PAGE_WIDTH_DEFAULT ? null : width });
        layout.dragWidth = null;
        dragSide = null;
      };
      el.addEventListener('pointermove', move);
      el.addEventListener('pointerup', up);
      el.addEventListener('pointercancel', up);
    };
  }
  const reset = () => updateSettings({ pageWidth: null });
</script>

<svelte:window onmousedown={writeBelow} />

<main class="stage" class:aside={app.chatOpen && !app.sidebarOpen && layout.vw >= 1100} class:shifted={app.sidebarOpen} class:dim={app.chatOpen && !chat.pinned}>
  <!-- Set on the page itself: a width variable on the root restyles the whole document, every sketch included, on each drag step. -->
  <div class="page" class:dragging={dragSide !== null} bind:this={page} style:width="min({pageWidth()}px, calc(100vw - 48px))">
    {#each [-1, 1] as const as side (side)}
      <div
        class="handle"
        class:left={side === -1}
        class:right={side === 1}
        class:active={dragSide === side}
        role="separator"
        aria-orientation="vertical"
        aria-label="Page width: drag to change, double-click to reset"
        aria-valuenow={pageWidth()}
        aria-valuemin={PAGE_WIDTH_MIN}
        aria-valuemax={PAGE_WIDTH_MAX}
        onpointerdown={drag(side)}
        ondblclick={reset}
      >
        <span class="line" aria-hidden="true"></span>
        {#if dragSide === side}<span class="readout" aria-hidden="true">{layout.dragWidth}px</span>{/if}
      </div>
    {/each}
    <!-- Holds the resize cursor over everything while dragging; a class on the root would restyle the whole document. -->
    {#if dragSide !== null}<div class="drag-cursor" aria-hidden="true"></div>{/if}

    {#if app.note}
      <NoteHeaderRow />
      {#if app.tellOpen}
        {#key app.note.id}<TellCard />{/key}
      {:else if app.note.brief}
        <button class="brief" data-tell onclick={() => (app.tellOpen = true)} title="Edit what {riddlerName()} keeps in mind"><span class="label"><Pencil size={13} strokeWidth={1.5} />Tell {riddlerName()}</span><span class="text">{app.note.brief}</span></button>
      {/if}
      <div class="title-gap" class:has-meta={app.tellOpen || app.note.brief}></div>
      <EditorView
        note={app.note}
        onReady={(a) => (api = a)}
        onEdit={() => { riddle?.onEdit(); interruptSpeech(); }}
        onBlocksChanged={(t) => riddle?.onBlocksChanged(t)}
        changeDelay={() => riddle?.delay() ?? 900}
      />
      {#if backlinks.length > 0}
        <footer class="backlinks" aria-label="Backlinks">
          <span class="backlinks-label">Mentioned in</span>
          {#each backlinks as link, i (link.id)}
            {#if i > 0}<span class="backlinks-sep" aria-hidden="true">·</span>{/if}
            <button class="backlink" onclick={() => void openNote(link.id)}>{link.title || 'Untitled'}</button>
          {/each}
        </footer>
      {/if}
    {/if}
    {#if api && page}
      <!-- Before the margin, so Tab goes on from the toolbar to the margin. -->
      <SelectionMenu editor={api.editor} {page} />
      <Margin />
      <ReadingPencil editor={api.editor} {page} />
    {/if}
  </div>
</main>

<style>
  .stage {
    min-height: 100vh;
    transition: transform var(--dur-drawer) var(--ease-drawer), opacity var(--dur-drawer) var(--ease-drawer);
  }
  .stage.dim { opacity: 0.5; }
  .stage.aside { transform: translateX(-210px); }
  .stage.shifted { transform: translateX(288px); opacity: 0.6; }
  .page {
    position: relative;
    margin: 0 auto;
    padding-top: 106px;
    padding-bottom: 34vh;
  }
  .handle {
    position: absolute;
    top: 61px;
    z-index: 20;
    display: flex;
    justify-content: center;
    width: 24px;
    height: 28px;
    cursor: var(--cursor-col-resize);
    touch-action: none;
    user-select: none;
  }
  .drag-cursor { position: fixed; inset: 0; z-index: 1000; cursor: var(--cursor-col-resize); }
  .handle.left { left: -12px; }
  .handle.right { right: -12px; }
  .line {
    margin-top: 4px;
    width: 2px;
    height: 14px;
    border-radius: 999px;
    background: var(--rule);
    transition: background-color var(--dur-ui) var(--ease-ui);
  }
  .handle:hover .line { background: var(--graphite); }
  .handle.active .line { background: var(--ink); }
  .readout {
    position: absolute;
    top: 0;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--ink);
    color: var(--paper);
    font-size: 11.5px;
    line-height: 1.4;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    pointer-events: none;
  }
  .handle.right .readout { right: 24px; }
  .handle.left .readout { left: 24px; }
  @media (pointer: coarse) { .handle { display: none; } }

  .brief {
    display: flex;
    align-items: baseline;
    gap: 12px;
    max-width: 100%;
    margin-top: 10px;
    text-align: left;
  }
  .label {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    line-height: 1.4;
    color: var(--faint);
    transform: translateY(1px);
    transition: color var(--dur-ui) var(--ease-ui);
  }
  .text {
    font-family: var(--font-voice);
    font-size: 17px;
    line-height: 1.45;
    color: var(--graphite);
    transition: color var(--dur-ui) var(--ease-ui);
  }
  .brief:hover .label, .brief:hover .text { color: var(--ink); }
  .title-gap { height: 12px; }
  .title-gap.has-meta { height: 4px; }

  .backlinks {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 8px;
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--rule);
    font-size: 13px;
    line-height: 1.5;
  }
  .backlinks-label {
    color: var(--faint);
    margin-right: 2px;
  }
  .backlinks-sep {
    color: var(--faint);
    user-select: none;
  }
  .backlink {
    color: var(--graphite);
    cursor: var(--cursor-pointer);
    text-decoration: underline;
    text-decoration-color: transparent;
    text-underline-offset: 3px;
    text-decoration-thickness: 1px;
    transition: color var(--dur-ui) var(--ease-ui), text-decoration-color var(--dur-ui) var(--ease-ui);
  }
  .backlink:hover {
    color: var(--ink);
    text-decoration-color: var(--rule);
  }

  @media (prefers-reduced-motion: reduce) {
    .stage { transition: opacity var(--dur-drawer) var(--ease-drawer); }
  }
</style>
