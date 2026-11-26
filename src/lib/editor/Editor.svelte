<script lang="ts" module>
  import type { Editor as TipTapEditor } from '@tiptap/core';
  import type { Range } from './quote';

  /** An edit that settled: every top-level block's text, title first, and the indexes of the blocks that are new. */
  export interface BlockChange {
    texts: string[];
    changed: number[];
  }

  export interface EditorApi {
    findQuote(quote: string): Range | null;
    editor: TipTapEditor;
  }
</script>

<script lang="ts">
  import { untrack } from 'svelte';
  import FindBar from '../components/FindBar.svelte';
  import { openNote, registerFlush, type OpenNote } from '../state/app.svelte';
  import { keepWords, storeNote } from '../state/saves';
  import { settings } from '../state/settings.svelte';
  import { showToast } from '../state/toast.svelte';
  import { createEditor } from './create';
  import { attachFindEditor, find } from './find.svelte';
  import NoteLinkMenu from './NoteLinkMenu.svelte';
  import { changedBlocks, docBlocks, normalizeDoc, withoutLastLine } from './plain';
  import { findQuote } from './quote';
  import SlashMenu from './SlashMenu.svelte';
  import { resetVimMode, vim } from './vim.svelte';

  interface Props {
    note: OpenNote;
    onBlocksChanged?: (change: BlockChange) => void;
    onEdit?: () => void;
    changeDelay?: () => number;
    onReady?: (api: EditorApi | null) => void;
  }
  let { note, onBlocksChanged, onEdit, changeDelay = () => 900, onReady }: Props = $props();

  const SAVE_DEBOUNCE = 600;
  let element: HTMLDivElement;
  let editor = $state.raw<TipTapEditor>();
  const noteId = $derived(note.id);
  const noteDoc = $derived(note.doc);

  function mount(n: OpenNote) {
    resetVimMode('normal');
    const id = n.id;
    const doc = normalizeDoc(n.doc, n.title);
    let timer: number | undefined;
    let changeTimer: number | undefined;
    let dirty = false;

    const save = () => {
      window.clearTimeout(timer);
      if (dirty) keepWords(id, withoutLastLine(ed.getJSON()));
      dirty = false;
      return storeNote(id);
    };
    // A save that fails is tried again by itself.
    const saveSoon = () => void save().catch(() => {});

    const flushChanges = () => {
      const texts = docBlocks(ed.state.doc);
      const changed = changedBlocks(lastBlocks, texts);
      lastBlocks = texts;
      if (changed.length) onBlocksChanged?.({ texts, changed });
    };

    const ed = createEditor({
      element,
      content: doc,
      autofocus: n.title ? 'end' : 'start',
      onUpdate: (_, quiet) => {
        dirty = true;
        if (!quiet) onEdit?.();
        window.clearTimeout(timer);
        timer = window.setTimeout(saveSoon, SAVE_DEBOUNCE);
        window.clearTimeout(changeTimer);
        changeTimer = window.setTimeout(flushChanges, changeDelay());
      },
    });
    let lastBlocks = docBlocks(ed.state.doc);
    editor = ed;
    attachFindEditor(ed);
    registerFlush(save);
    onReady?.({ editor: ed, findQuote: (quote) => findQuote(ed.state.doc, quote) });

    return () => {
      window.clearTimeout(changeTimer);
      attachFindEditor(null);
      registerFlush(null);
      onReady?.(null);
      saveSoon();
      ed.destroy();
      editor = undefined;
    };
  }

  // One element for the editor's whole life, one TipTap instance per note. Unmounting a focused
  // editor fires blur during Svelte's teardown, and TipTap's listeners then write dying state.
  $effect(() => {
    void noteId;
    void noteDoc;
    return untrack(() => mount(note));
  });

  $effect(() => {
    if (settings.value.vimMode) resetVimMode('normal');
  });

  function openLinkTarget(target: EventTarget | null): boolean {
    const link = (target as HTMLElement | null)?.closest<HTMLElement>('.note-link[data-note-link]');
    const id = link?.dataset.noteLink;
    if (!id) return false;
    void openNote(id).catch(() => showToast('That note no longer exists.'));
    return true;
  }

  $effect(() => {
    const el = element;
    if (!el) return;
    const onClick = (e: MouseEvent) => {
      if (e.button !== 0) return;
      if (openLinkTarget(e.target)) e.preventDefault();
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Enter' && e.key !== ' ') return;
      if (openLinkTarget(e.target)) e.preventDefault();
    };
    el.addEventListener('click', onClick);
    el.addEventListener('keydown', onKeyDown);
    return () => {
      el.removeEventListener('click', onClick);
      el.removeEventListener('keydown', onKeyDown);
    };
  });
</script>

<div class="editor" class:vim-normal={settings.value.vimMode && vim.mode === 'normal'} bind:this={element}></div>
{#if editor}
  <SlashMenu {editor} />
  <NoteLinkMenu {editor} />
{/if}
{#if find.open}
  <FindBar />
{/if}

<style>
  .editor {
    position: relative;
    min-height: 50vh;
    padding-bottom: 64px;
  }
  .editor :global(.ProseMirror .find-match) {
    background: var(--riddle-wash);
    border-radius: 2px;
  }
  .editor :global(.ProseMirror .find-match-current) {
    background: color-mix(in srgb, var(--bulb) 35%, transparent);
    border-radius: 2px;
  }
  .editor.vim-normal :global(.ProseMirror:not(.drawn-caret)) {
    caret-color: var(--ink);
  }
  .editor.vim-normal :global(.caret) {
    width: 7px;
    border-radius: 1.5px;
    opacity: 0.28;
  }
</style>
