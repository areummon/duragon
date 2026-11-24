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
  import { getBackend } from '../api/backend';
  import type { Note } from '../api/types';
  import { registerFlush } from '../state/app.svelte';
  import { upsertMeta } from '../state/notes.svelte';
  import { createEditor } from './create';
  import { changedBlocks, docBlocks, docTitle, normalizeDoc, plainText, withoutLastLine } from './plain';
  import { findQuote } from './quote';
  import SlashMenu from './SlashMenu.svelte';

  interface Props {
    note: Note;
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

  async function persist(id: string, json: ReturnType<TipTapEditor['getJSON']>) {
    upsertMeta(await (await getBackend()).saveNote(id, docTitle(json), json, plainText(json)));
  }

  function mount(n: Note) {
    const id = n.id;
    const doc = normalizeDoc(n.doc, n.title);
    let timer: number | undefined;
    let changeTimer: number | undefined;
    let dirty = false;
    let saving: Promise<void> | undefined;

    // Waits for a save already on its way too, so a note never leaves before its last words are in.
    const save = async () => {
      window.clearTimeout(timer);
      if (dirty) {
        dirty = false;
        const mine = persist(id, withoutLastLine(ed.getJSON())).finally(() => {
          if (saving === mine) saving = undefined;
        });
        saving = mine;
      }
      await saving;
    };

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
        timer = window.setTimeout(() => void save(), SAVE_DEBOUNCE);
        window.clearTimeout(changeTimer);
        changeTimer = window.setTimeout(flushChanges, changeDelay());
      },
    });
    let lastBlocks = docBlocks(ed.state.doc);
    editor = ed;
    registerFlush(save);
    onReady?.({ editor: ed, findQuote: (quote) => findQuote(ed.state.doc, quote) });

    return () => {
      window.clearTimeout(timer);
      window.clearTimeout(changeTimer);
      registerFlush(null);
      onReady?.(null);
      const last = dirty ? withoutLastLine(ed.getJSON()) : null;
      ed.destroy();
      editor = undefined;
      if (last) void persist(id, last);
    };
  }

  // One element for the editor's whole life, one TipTap instance per note. Unmounting a focused
  // editor fires blur during Svelte's teardown, and TipTap's listeners then write dying state.
  $effect(() => {
    void noteId;
    return untrack(() => mount(note));
  });
</script>

<div class="editor" bind:this={element}></div>
{#if editor}
  <SlashMenu {editor} />
{/if}

<style>
  .editor {
    position: relative;
    min-height: 60vh;
    padding-bottom: 40vh;
  }
</style>
