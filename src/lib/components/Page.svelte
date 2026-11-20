<script lang="ts">
  import EditorView, { type EditorApi } from '../editor/Editor.svelte';
  import { app } from '../state/app.svelte';
  import { riddlerName } from '../state/riddler';
  import {
    PAGE_WIDTH_DEFAULT, PAGE_WIDTH_MAX, PAGE_WIDTH_MIN, previewPageWidth, settings, updateSettings,
  } from '../state/settings.svelte';
  import { attachRiddle, type Riddle } from '../editor/riddle.svelte';
  import { chat } from '../state/chat.svelte';
  import { loadThreads, unloadThreads } from '../state/threads.svelte';
  import { Pencil } from 'lucide-svelte';
  import { interruptSpeech } from '../state/voice.svelte';
  import Margin from './Margin.svelte';
  import SelectionMenu from './SelectionMenu.svelte';
  import NoteHeaderRow from './NoteHeaderRow.svelte';
  import TellCard from './TellCard.svelte';

  let api = $state.raw<EditorApi | null>(null);
  let page = $state<HTMLElement>();
  let riddle = $state.raw<Riddle | null>(null);

  $effect(() => {
    const id = app.note?.id;
    if (!id) return;
    void loadThreads(id);
    return unloadThreads;
  });

  $effect(() => {
    if (!api || !page || !app.note) return;
    const r = attachRiddle(api.editor, page, app.note.id);
    riddle = r;
    return () => {
      r.detach();
      riddle = null;
    };
  });

  $effect(() => {
    if (!app.focusTitle || !api) return;
    app.focusTitle = false;
    const title = api.editor.state.doc.firstChild!;
    api.editor.chain().focus().setTextSelection({ from: 1, to: 1 + title.content.size }).run();
  });

  function drag(side: -1 | 1) {
    return (e: PointerEvent) => {
      if (e.button !== 0) return;
      const el = e.currentTarget as HTMLElement;
      el.setPointerCapture(e.pointerId);
      const startX = e.clientX;
      const start = page!.getBoundingClientRect().width;
      let width = start;
      const move = (m: PointerEvent) => {
        width = Math.round(Math.min(PAGE_WIDTH_MAX, Math.max(PAGE_WIDTH_MIN, start + side * 2 * (m.clientX - startX))));
        previewPageWidth(width);
      };
      const up = () => {
        el.removeEventListener('pointermove', move);
        el.removeEventListener('pointerup', up);
        el.removeEventListener('pointercancel', up);
        void updateSettings({ pageWidth: width === PAGE_WIDTH_DEFAULT ? null : width });
      };
      el.addEventListener('pointermove', move);
      el.addEventListener('pointerup', up);
      el.addEventListener('pointercancel', up);
    };
  }
  const reset = () => updateSettings({ pageWidth: null });
</script>

<main class="stage" class:shifted={app.sidebarOpen} class:dim={app.chatOpen && !chat.pinned}>
  <div class="page" bind:this={page}>
    <div class="handle left" role="separator" aria-orientation="vertical" aria-label="Page width" title="Drag to change page width, double-click to reset" onpointerdown={drag(-1)} ondblclick={reset}></div>
    <div class="handle right" role="separator" aria-orientation="vertical" aria-label="Page width" title="Drag to change page width, double-click to reset" onpointerdown={drag(1)} ondblclick={reset}></div>

    {#if app.note}
      <NoteHeaderRow />
      {#if app.tellOpen}
        {#key app.note.id}<TellCard />{/key}
      {:else if app.note.brief}
        <button class="brief" onclick={() => (app.tellOpen = true)} title="Edit what {riddlerName()} keeps in mind"><span class="label"><Pencil size={13} strokeWidth={1.5} />Tell {riddlerName()}</span><span class="text">{app.note.brief}</span></button>
      {/if}
      {#key app.note.id}
        <div class="title-gap" class:has-meta={app.tellOpen || app.note.brief}></div>
        <EditorView
          note={app.note}
          onReady={(a) => (api = a)}
          onEdit={() => { riddle?.onEdit(); interruptSpeech(); }}
          onBlocksChanged={(t) => riddle?.onBlocksChanged(t)}
          changeDelay={() => riddle?.delay() ?? 900}
        />
      {/key}
    {/if}
    {#if api && page}
      <Margin />
      <SelectionMenu editor={api.editor} {page} />
    {/if}
  </div>
</main>

<style>
  .stage {
    min-height: 100vh;
    transition: transform 320ms var(--ease-step), opacity 320ms var(--ease-step);
  }
  .stage.dim { opacity: 0.6; }
  .stage.shifted { transform: translateX(288px); opacity: 0.6; }
  .page {
    position: relative;
    width: min(var(--page-width), calc(100vw - 48px));
    margin: 0 auto;
    padding-top: 106px;
  }
  .handle {
    position: absolute;
    top: 63px;
    width: 14px;
    height: 24px;
    cursor: ew-resize;
    touch-action: none;
  }
  .handle.left { left: -7px; }
  .handle.right { right: -7px; }
  .handle::after {
    content: '';
    position: absolute;
    top: 6px;
    left: 6px;
    width: 1px;
    height: 12px;
    background: var(--faint);
    opacity: 0.7;
    transition: opacity var(--dur-ui) var(--ease-ui);
  }
  .handle:hover::after { opacity: 1; }

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
</style>
