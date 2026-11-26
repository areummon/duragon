import { Extension, type Editor } from '@tiptap/core';
import type { Node as PMNode, ResolvedPos } from '@tiptap/pm/model';
import { Plugin, PluginKey, Selection, TextSelection, type EditorState } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';
import { app, toggleChat, toggleJustMyWords, updateMeta } from '../state/app.svelte';
import { settings } from '../state/settings.svelte';
import { slash } from './slash.svelte';

export type VimMode = 'normal' | 'insert' | 'visual';
export const vim = $state<{ mode: VimMode; pending: string }>({ mode: 'normal', pending: '' });

export function resetVimMode(mode: VimMode = 'normal') {
  vim.mode = mode;
  vim.pending = '';
}

const yankRegister: { text: string; linewise: boolean } = { text: '', linewise: false };

function yank(text: string, linewise: boolean) {
  yankRegister.text = text;
  yankRegister.linewise = linewise;
  void navigator.clipboard?.writeText(text).catch(() => {});
}

interface TextblockSpan {
  start: number;
  end: number;
  node: PMNode;
}

const isEmptyParagraph = (node: PMNode) => node.type.name === 'paragraph' && node.content.size === 0;

function collectTextblocks(doc: PMNode): TextblockSpan[] {
  const blocks: TextblockSpan[] = [];
  doc.descendants((node, pos) => {
    if (node.isTextblock) {
      blocks.push({ start: pos + 1, end: pos + 1 + node.content.size, node });
      return false;
    }
    return true;
  });
  return blocks;
}

function parentText(resolved: ResolvedPos): string {
  return resolved.parent.textBetween(0, resolved.parent.content.size, '\n', '\ufffc');
}

function charClass(ch: string): 'space' | 'word' | 'punct' {
  if (/\s/.test(ch)) return 'space';
  if (/[A-Za-z0-9_]/.test(ch)) return 'word';
  return 'punct';
}

function firstNonSpacePos(resolved: ResolvedPos): number {
  const start = resolved.start(resolved.depth);
  const idx = parentText(resolved).search(/\S/);
  return idx === -1 ? start : start + idx;
}

function adjacentTextblock(doc: PMNode, pos: number, dir: 1 | -1): TextblockSpan | null {
  const blocks = collectTextblocks(doc);
  const idx = blocks.findIndex((b) => pos >= b.start && pos <= b.end);
  if (idx === -1) return blocks[0] ?? null;
  return blocks[idx + dir] ?? null;
}

function nextWordStart(doc: PMNode, pos: number, crossBlock: boolean): number {
  const resolved = doc.resolve(pos);
  const start = resolved.start(resolved.depth);
  const end = resolved.end(resolved.depth);
  const text = parentText(resolved);
  let i = resolved.parentOffset;

  if (i < text.length) {
    const cls = charClass(text[i]);
    if (cls !== 'space') {
      while (i < text.length && charClass(text[i]) === cls) i++;
    }
    while (i < text.length && charClass(text[i]) === 'space') i++;
    if (i < text.length) return start + i;
  }

  if (!crossBlock) return end;
  const next = adjacentTextblock(doc, pos, 1);
  if (!next) return end;
  const nextText = next.node.textBetween(0, next.node.content.size, '\n', '\ufffc');
  const first = nextText.search(/\S/);
  return first === -1 ? next.start : next.start + first;
}

function changeWordEnd(doc: PMNode, pos: number): number {
  const resolved = doc.resolve(pos);
  const start = resolved.start(resolved.depth);
  const end = resolved.end(resolved.depth);
  const text = parentText(resolved);
  let i = resolved.parentOffset;
  if (i >= text.length) return end;

  const cls = charClass(text[i]);
  while (i < text.length && charClass(text[i]) === cls) i++;
  return start + i;
}

function prevWordStart(doc: PMNode, pos: number): number {
  const resolved = doc.resolve(pos);
  const start = resolved.start(resolved.depth);
  const text = parentText(resolved);
  let i = resolved.parentOffset - 1;

  while (i >= 0 && charClass(text[i]) === 'space') i--;
  if (i >= 0) {
    const cls = charClass(text[i]);
    while (i > 0 && charClass(text[i - 1]) === cls) i--;
    return start + i;
  }

  const prev = adjacentTextblock(doc, pos, -1);
  if (!prev) return start;
  const prevText = prev.node.textBetween(0, prev.node.content.size, '\n', '\ufffc');
  let j = prevText.length - 1;
  while (j >= 0 && charClass(prevText[j]) === 'space') j--;
  if (j < 0) return prev.start;
  const cls = charClass(prevText[j]);
  while (j > 0 && charClass(prevText[j - 1]) === cls) j--;
  return prev.start + j;
}

function nextWordEnd(doc: PMNode, pos: number): number {
  const resolved = doc.resolve(pos);
  const start = resolved.start(resolved.depth);
  const end = resolved.end(resolved.depth);
  const text = parentText(resolved);
  let i = resolved.parentOffset + 1;

  while (i < text.length && charClass(text[i]) === 'space') i++;
  if (i < text.length) {
    const cls = charClass(text[i]);
    while (i + 1 < text.length && charClass(text[i + 1]) === cls) i++;
    return start + i;
  }

  const next = adjacentTextblock(doc, pos, 1);
  if (!next) return end;
  const nextText = next.node.textBetween(0, next.node.content.size, '\n', '\ufffc');
  let j = 0;
  while (j < nextText.length && charClass(nextText[j]) === 'space') j++;
  if (j >= nextText.length) return next.start;
  const cls = charClass(nextText[j]);
  while (j + 1 < nextText.length && charClass(nextText[j + 1]) === cls) j++;
  return next.start + j;
}

function verticalMove(view: EditorView, pos: number, dir: 1 | -1): number {
  const { doc } = view.state;
  const resolved = doc.resolve(pos);
  const blockStart = resolved.start(resolved.depth);
  const blockEnd = resolved.end(resolved.depth);

  // Try stepping one visual line inside a wrapped textblock first.
  try {
    const cur = view.coordsAtPos(pos);
    const probeTop = dir > 0 ? cur.bottom + 4 : cur.top - 4;
    const hit = view.posAtCoords({ left: cur.left, top: probeTop });
    if (hit && hit.pos >= blockStart && hit.pos <= blockEnd && hit.pos !== pos) {
      const target = view.coordsAtPos(hit.pos);
      if (dir > 0 ? target.top > cur.top + 2 : target.bottom < cur.bottom - 2) {
        return hit.pos;
      }
    }
  } catch {
    // Fall back to block-level movement if coordinates are unavailable.
  }

  const adj = adjacentTextblock(doc, pos, dir);
  if (!adj) return pos;
  return Math.min(adj.end, adj.start + resolved.parentOffset);
}

function topOfDoc(state: EditorState): number {
  return Selection.atStart(state.doc).from;
}

function bottomOfDoc(state: EditorState): number {
  const { doc } = state;
  const blocks = collectTextblocks(doc);
  if (blocks.length === 0) return Selection.atEnd(doc).from;

  // Respect Duragon's trailing LastLine empty paragraph.
  let lastIdx = blocks.length - 1;
  if (lastIdx > 1 && isEmptyParagraph(blocks[lastIdx].node)) {
    lastIdx--;
  }
  // Walk back to the last non-empty body block when possible.
  for (let i = lastIdx; i >= 1; i--) {
    if (blocks[i].node.content.size > 0) return blocks[i].start;
  }
  return blocks[lastIdx].start;
}

function resolveMotion(view: EditorView, pos: number, key: string): number | null {
  const { state } = view;
  const { doc } = state;
  const resolved = doc.resolve(pos);
  const start = resolved.start(resolved.depth);
  const end = resolved.end(resolved.depth);

  switch (key) {
    case 'h':
      return Math.max(start, pos - 1);
    case 'l':
      return Math.min(end, pos + 1);
    case 'j':
      return verticalMove(view, pos, 1);
    case 'k':
      return verticalMove(view, pos, -1);
    case 'w':
      return nextWordStart(doc, pos, true);
    case 'b':
      return prevWordStart(doc, pos);
    case 'e':
      return nextWordEnd(doc, pos);
    case '0':
      return start;
    case '^':
      return firstNonSpacePos(resolved);
    case '$':
      return end;
    case 'G':
      return bottomOfDoc(state);
    default:
      return null;
  }
}

function moveCursor(view: EditorView, target: number) {
  const tr = view.state.tr.setSelection(TextSelection.create(view.state.doc, target)).scrollIntoView();
  view.dispatch(tr);
}

function extendVisual(view: EditorView, target: number) {
  const { doc, selection } = view.state;
  const tr = view.state.tr.setSelection(TextSelection.create(doc, selection.anchor, target)).scrollIntoView();
  view.dispatch(tr);
}

function exitToNormal(view: EditorView) {
  const { state } = view;
  const { selection } = state;
  const prevMode = vim.mode;
  vim.mode = 'normal';
  vim.pending = '';

  if (prevMode === 'insert') {
    const fromPos = selection.$from;
    const pos = fromPos.parentOffset > 0 ? selection.from - 1 : selection.from;
    moveCursor(view, pos);
  } else if (prevMode === 'visual') {
    moveCursor(view, selection.head);
  }
}

function openLineBelow(view: EditorView) {
  const { state } = view;
  const { doc, schema, selection } = state;
  const fromPos = selection.$from;
  const insertPos = fromPos.parent.type.name === 'title' ? doc.firstChild!.nodeSize : fromPos.after(1);
  const tr = state.tr.insert(insertPos, schema.nodes.paragraph.create());
  tr.setSelection(TextSelection.create(tr.doc, insertPos + 1)).scrollIntoView();
  view.dispatch(tr);
  vim.mode = 'insert';
  vim.pending = '';
}

function openLineAbove(view: EditorView) {
  const { state } = view;
  const { schema, selection } = state;
  const fromPos = selection.$from;
  if (fromPos.parent.type.name === 'title') {
    moveCursor(view, fromPos.start(fromPos.depth));
    vim.mode = 'insert';
    vim.pending = '';
    return;
  }
  const insertPos = fromPos.before(1);
  const tr = state.tr.insert(insertPos, schema.nodes.paragraph.create());
  tr.setSelection(TextSelection.create(tr.doc, insertPos + 1)).scrollIntoView();
  view.dispatch(tr);
  vim.mode = 'insert';
  vim.pending = '';
}

function deleteCurrentBlock(view: EditorView) {
  const { state } = view;
  const { doc, selection } = state;
  const fromPos = selection.$from;
  const start = fromPos.start(fromPos.depth);
  const end = fromPos.end(fromPos.depth);
  yank(parentText(fromPos), true);

  // Never delete the Title node; clear its text instead.
  if (fromPos.parent.type.name === 'title') {
    if (end > start) view.dispatch(state.tr.delete(start, end).scrollIntoView());
    return;
  }

  // Keep NoteDocument's `title block+` invariant and Duragon's trailing LastLine valid.
  const isTrailingLastLine = fromPos.depth === 1 && fromPos.index(0) === doc.childCount - 1 && isEmptyParagraph(fromPos.parent);
  if (isTrailingLastLine) return;

  if (doc.childCount <= 2) {
    if (end > start) view.dispatch(state.tr.delete(start, end).scrollIntoView());
    return;
  }

  const from = fromPos.before(1);
  const to = fromPos.after(1);
  const tr = state.tr.delete(from, to);
  const target = Math.min(from, tr.doc.content.size);
  tr.setSelection(Selection.near(tr.doc.resolve(target))).scrollIntoView();
  view.dispatch(tr);
}

function changeCurrentBlock(view: EditorView) {
  const { state } = view;
  const fromPos = state.selection.$from;
  const start = fromPos.start(fromPos.depth);
  const end = fromPos.end(fromPos.depth);
  yank(parentText(fromPos), true);
  if (end > start) {
    view.dispatch(state.tr.delete(start, end).scrollIntoView());
  }
  vim.mode = 'insert';
  vim.pending = '';
}

function deleteRangeAndYank(view: EditorView, from: number, to: number, enterInsert: boolean) {
  const { state } = view;
  if (to > from) {
    yank(state.doc.textBetween(from, to, '\n'), false);
    view.dispatch(state.tr.delete(from, to).scrollIntoView());
  }
  if (enterInsert) {
    vim.mode = 'insert';
    vim.pending = '';
  }
}

function pasteRegister(view: EditorView, before: boolean) {
  if (!yankRegister.text && !yankRegister.linewise) return;
  const { state } = view;
  const { doc, schema, selection } = state;
  const fromPos = selection.$from;

  if (yankRegister.linewise) {
    const inTitle = fromPos.parent.type.name === 'title';
    if (before && inTitle) return;
    const insertPos = before
      ? fromPos.before(1)
      : inTitle
        ? doc.firstChild!.nodeSize
        : fromPos.after(1);
    const para = yankRegister.text
      ? schema.nodes.paragraph.create(null, schema.text(yankRegister.text))
      : schema.nodes.paragraph.create();
    const tr = state.tr.insert(insertPos, para);
    tr.setSelection(TextSelection.create(tr.doc, insertPos + 1)).scrollIntoView();
    view.dispatch(tr);
    return;
  }

  const end = fromPos.end(fromPos.depth);
  const at = before ? selection.from : Math.min(end, selection.from + 1);
  const tr = state.tr.insertText(yankRegister.text, at);
  const caret = Math.max(at, at + yankRegister.text.length - 1);
  tr.setSelection(TextSelection.create(tr.doc, caret)).scrollIntoView();
  view.dispatch(tr);
}

const PASS_THROUGH_KEYS = new Set([
  'ArrowLeft',
  'ArrowRight',
  'ArrowUp',
  'ArrowDown',
  'Home',
  'End',
  'PageUp',
  'PageDown',
  'Tab',
]);

const MODIFIER_KEYS = new Set(['Shift', 'Control', 'Alt', 'Meta', 'CapsLock', 'Dead']);

function handleVisualKey(view: EditorView, key: string, event: KeyboardEvent): boolean {
  if (vim.pending === 'g') {
    vim.pending = '';
    event.preventDefault();
    if (key === 'g') extendVisual(view, topOfDoc(view.state));
    return true;
  }

  if (key === 'g') {
    vim.pending = 'g';
    event.preventDefault();
    return true;
  }

  if (key === 'v') {
    event.preventDefault();
    exitToNormal(view);
    return true;
  }

  const motionTarget = resolveMotion(view, view.state.selection.head, key);
  if (motionTarget !== null) {
    event.preventDefault();
    extendVisual(view, motionTarget);
    return true;
  }

  const { state } = view;
  const { selection } = state;

  if (key === 'd' || key === 'x') {
    event.preventDefault();
    if (!selection.empty) {
      yank(state.doc.textBetween(selection.from, selection.to, '\n'), false);
      view.dispatch(state.tr.deleteSelection().scrollIntoView());
    }
    vim.mode = 'normal';
    vim.pending = '';
    return true;
  }

  if (key === 'c') {
    event.preventDefault();
    if (!selection.empty) {
      yank(state.doc.textBetween(selection.from, selection.to, '\n'), false);
      view.dispatch(state.tr.deleteSelection().scrollIntoView());
    }
    vim.mode = 'insert';
    vim.pending = '';
    return true;
  }

  if (key === 'y') {
    event.preventDefault();
    if (!selection.empty) {
      yank(state.doc.textBetween(selection.from, selection.to, '\n'), false);
    }
    vim.mode = 'normal';
    vim.pending = '';
    moveCursor(view, selection.from);
    return true;
  }

  if (PASS_THROUGH_KEYS.has(key)) return false;

  if (key.length === 1 || key === 'Backspace' || key === 'Delete' || key === 'Enter') {
    event.preventDefault();
    return true;
  }

  return false;
}

function handlePendingNormal(view: EditorView, pending: string, key: string) {
  const { state } = view;
  const { doc, selection } = state;
  const pos = selection.from;
  const fromPos = selection.$from;
  const end = fromPos.end(fromPos.depth);

  if (pending === 'g') {
    if (key === 'g') moveCursor(view, topOfDoc(state));
    else if (key === 'c') toggleChat();
    else if (key === 'w' && app.note) void updateMeta({ webOn: !app.note.webOn });
    else if (key === 'v' && app.note) void updateMeta({ voiceOn: !app.note.voiceOn });
    else if (key === 'm' && app.note) void updateMeta({ mode: app.note.mode === 'thinks' ? 'asks' : 'thinks' });
    return;
  }

  if (pending === 'd') {
    if (key === 'd') deleteCurrentBlock(view);
    else if (key === 'w') deleteRangeAndYank(view, pos, nextWordStart(doc, pos, false), false);
    else if (key === '$') deleteRangeAndYank(view, pos, end, false);
    return;
  }

  if (pending === 'c') {
    if (key === 'c') changeCurrentBlock(view);
    else if (key === 'w') deleteRangeAndYank(view, pos, changeWordEnd(doc, pos), true);
    else if (key === '$') deleteRangeAndYank(view, pos, end, true);
    return;
  }

  if (pending === 'y') {
    if (key === 'y') yank(parentText(fromPos), true);
    return;
  }

  if (pending === 'z') {
    if (key === 'a') void toggleJustMyWords();
  }
}

function handleNormalKey(editor: Editor, view: EditorView, key: string, event: KeyboardEvent): boolean {
  if (vim.pending) {
    const pending = vim.pending;
    vim.pending = '';
    event.preventDefault();
    handlePendingNormal(view, pending, key);
    return true;
  }

  if (key === 'g' || key === 'd' || key === 'c' || key === 'y' || key === 'z') {
    vim.pending = key;
    event.preventDefault();
    return true;
  }

  const { state } = view;
  const { selection } = state;
  const pos = selection.from;
  const fromPos = selection.$from;
  const end = fromPos.end(fromPos.depth);

  switch (key) {
    case 'i':
      event.preventDefault();
      vim.mode = 'insert';
      return true;
    case 'a':
      event.preventDefault();
      moveCursor(view, Math.min(end, pos + 1));
      vim.mode = 'insert';
      return true;
    case 'I':
      event.preventDefault();
      moveCursor(view, firstNonSpacePos(fromPos));
      vim.mode = 'insert';
      return true;
    case 'A':
      event.preventDefault();
      moveCursor(view, end);
      vim.mode = 'insert';
      return true;
    case 'o':
      event.preventDefault();
      openLineBelow(view);
      return true;
    case 'O':
      event.preventDefault();
      openLineAbove(view);
      return true;
    case 'v':
      event.preventDefault();
      vim.mode = 'visual';
      vim.pending = '';
      moveCursor(view, pos);
      return true;
    case 'x':
      event.preventDefault();
      if (pos < end) deleteRangeAndYank(view, pos, pos + 1, false);
      return true;
    case 'D':
      event.preventDefault();
      deleteRangeAndYank(view, pos, end, false);
      return true;
    case 'C':
      event.preventDefault();
      deleteRangeAndYank(view, pos, end, true);
      return true;
    case 'p':
      event.preventDefault();
      pasteRegister(view, false);
      return true;
    case 'P':
      event.preventDefault();
      pasteRegister(view, true);
      return true;
    case 'u':
      event.preventDefault();
      editor.commands.undo();
      return true;
    default:
      break;
  }

  const motionTarget = resolveMotion(view, pos, key);
  if (motionTarget !== null) {
    event.preventDefault();
    moveCursor(view, motionTarget);
    return true;
  }

  if (PASS_THROUGH_KEYS.has(key)) return false;

  if (key.length === 1 || key === 'Backspace' || key === 'Delete' || key === 'Enter') {
    vim.pending = '';
    event.preventDefault();
    return true;
  }

  return false;
}

const vimPluginKey = new PluginKey('vimMode');

export const VimMode = Extension.create({
  name: 'vimMode',
  priority: 2000,

  addProseMirrorPlugins() {
    const editor = this.editor;
    return [
      new Plugin({
        key: vimPluginKey,
        props: {
          handleKeyDown(view, event) {
            if (!settings.value.vimMode) return false;
            if (event.isComposing) return false;
            if (slash.open) return false;

            const key = event.key;
            if (MODIFIER_KEYS.has(key)) return false;

            if (event.metaKey || event.ctrlKey || event.altKey) {
              if (event.ctrlKey && !event.metaKey && !event.altKey) {
                if (key === 'r' && vim.mode === 'normal') {
                  event.preventDefault();
                  vim.pending = '';
                  editor.commands.redo();
                  return true;
                }
                if (key === '[' && (vim.mode === 'insert' || vim.mode === 'visual')) {
                  event.preventDefault();
                  event.stopPropagation();
                  exitToNormal(view);
                  return true;
                }
              }
              return false;
            }

            if (key === 'Escape') {
              if (vim.mode === 'insert' || vim.mode === 'visual') {
                event.preventDefault();
                event.stopPropagation();
                exitToNormal(view);
                return true;
              }
              if (vim.pending) {
                event.preventDefault();
                event.stopPropagation();
                vim.pending = '';
                return true;
              }
              return false;
            }

            if (vim.mode === 'insert') return false;
            if (vim.mode === 'visual') return handleVisualKey(view, key, event);
            return handleNormalKey(editor, view, key, event);
          },
        },
      }),
    ];
  },
});

if (import.meta.env.DEV) {
  const win = window as unknown as { __duragon?: Record<string, unknown> };
  win.__duragon ??= {};
  win.__duragon.vim = vim;
}
