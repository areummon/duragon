<script lang="ts" module>
  import type { Editor as TipTapEditor } from '@tiptap/core';
  import type { Range } from './quote';

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
  import { blockTexts, docTitle, normalizeDoc, plainText } from './plain';
  import { findQuote } from './quote';
  import SlashMenu from './SlashMenu.svelte';

  interface Props {
    note: Note;
    focusTitle?: boolean;
    onBlocksChanged?: (changedBlockTexts: string[]) => void;
    onEdit?: () => void;
    changeDelay?: () => number;
    onReady?: (api: EditorApi | null) => void;
  }
  let { note, focusTitle = false, onBlocksChanged, onEdit, changeDelay = () => 900, onReady }: Props = $props();

  const SAVE_DEBOUNCE = 600;
  let element: HTMLDivElement;
  let editor = $state.raw<TipTapEditor>();
  const noteId = $derived(note.id);

  async function persist(id: string, json: ReturnType<TipTapEditor['getJSON']>) {
    upsertMeta(await (await getBackend()).saveNote(id, docTitle(json), json, plainText(json)));
  }

  /** Blocks of `current` (title excluded) that were not in `before`, counting duplicates. */
  function changed(before: string[], current: string[]): string[] {
    const left = new Map<string, number>();
    before.forEach((t) => left.set(t, (left.get(t) ?? 0) + 1));
    return current.slice(1).filter((t) => {
      const n = left.get(t) ?? 0;
      if (n > 0) {
        left.set(t, n - 1);
        return false;
      }
      return t.trim() !== '';
    });
  }

  function mount(n: Note) {
    const id = n.id;
    const doc = normalizeDoc(n.doc, n.title);
    let timer: number | undefined;
    let changeTimer: number | undefined;
    let dirty = false;
    let lastBlocks = blockTexts(doc).slice(1);

    const save = async () => {
      window.clearTimeout(timer);
      if (!dirty) return;
      dirty = false;
      await persist(id, ed.getJSON());
    };

    const flushChanges = () => {
      const current = blockTexts(ed.getJSON());
      const diff = changed(lastBlocks, current);
      lastBlocks = current.slice(1);
      if (diff.length) onBlocksChanged?.(diff);
    };

    const ed = createEditor({
      element,
      content: doc,
      autofocus: focusTitle || !n.title ? 'start' : 'end',
      onUpdate: (_, quiet) => {
        dirty = true;
        if (!quiet) onEdit?.();
        window.clearTimeout(timer);
        timer = window.setTimeout(() => void save(), SAVE_DEBOUNCE);
        window.clearTimeout(changeTimer);
        changeTimer = window.setTimeout(flushChanges, changeDelay());
      },
    });
    editor = ed;
    registerFlush(save);
    onReady?.({ editor: ed, findQuote: (quote) => findQuote(ed.state.doc, quote) });

    return () => {
      window.clearTimeout(timer);
      window.clearTimeout(changeTimer);
      registerFlush(null);
      onReady?.(null);
      const last = dirty ? ed.getJSON() : null;
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
