<script lang="ts" module>
  import type { Editor as TipTapEditor } from '@tiptap/core';
  import type { Range } from './quote';

  export interface EditorApi {
    findQuote(quote: string): Range | null;
    editor: TipTapEditor;
  }
</script>

<script lang="ts">
  import { onMount } from 'svelte';
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
  let timer: number | undefined;
  let changeTimer: number | undefined;
  let dirty = false;
  let lastBlocks: string[] = [];
  let editor = $state.raw<TipTapEditor>();
  // svelte-ignore state_referenced_locally
  const noteId = note.id;

  async function save() {
    window.clearTimeout(timer);
    timer = undefined;
    if (!editor || !dirty) return;
    dirty = false;
    const doc = editor.getJSON();
    const meta = await (await getBackend()).saveNote(noteId, docTitle(doc), doc, plainText(doc));
    upsertMeta(meta);
  }

  function changed(current: string[]): string[] {
    const before = new Map<string, number>();
    lastBlocks.forEach((t) => before.set(t, (before.get(t) ?? 0) + 1));
    return current.slice(1).filter((t) => {
      const n = before.get(t) ?? 0;
      if (n > 0) {
        before.set(t, n - 1);
        return false;
      }
      return t.trim() !== '';
    });
  }

  function flushChanges() {
    if (!editor) return;
    const current = blockTexts(editor.getJSON());
    const diff = changed(current);
    lastBlocks = current.slice(1);
    if (diff.length) onBlocksChanged?.(diff);
  }

  onMount(() => {
    const doc = normalizeDoc(note.doc, note.title);
    lastBlocks = blockTexts(doc).slice(1);
    editor = createEditor({
      element,
      content: doc,
      autofocus: focusTitle || !note.title ? 'start' : 'end',
      onUpdate: () => {
        dirty = true;
        onEdit?.();
        window.clearTimeout(timer);
        timer = window.setTimeout(() => void save(), SAVE_DEBOUNCE);
        window.clearTimeout(changeTimer);
        changeTimer = window.setTimeout(flushChanges, changeDelay());
      },
    });
    registerFlush(save);
    onReady?.({ editor, findQuote: (quote) => findQuote(editor!.state.doc, quote) });
    return () => {
      registerFlush(null);
      window.clearTimeout(changeTimer);
      onReady?.(null);
      void save();
      editor?.destroy();
    };
  });
</script>

<div class="editor" bind:this={element}></div>
{#if editor}
  <SlashMenu {editor} />
{/if}

<style>
  .editor {
    min-height: 60vh;
    padding-bottom: 40vh;
  }
</style>
