import { Extension, type Editor } from '@tiptap/core';
import type { ResolvedPos } from '@tiptap/pm/model';
import { Plugin, PluginKey, Selection, TextSelection } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';
import { exposeDebug } from '../debug';
import { app, openCustomize, toggleChat, toggleJustMyWords, updateMeta } from '../state/app.svelte';
import { settings } from '../state/settings.svelte';
import { isEmptyLine } from './create';
import { clearFindHighlights, find, nextMatch, openFind, prevMatch } from './find.svelte';
import { anyInlineMenuOpen } from './inline-menu.svelte';
import {
  changeWordEnd,
  firstNonSpacePos,
  nextWordStart,
  parentText,
  resolveMotion,
  topOfDoc,
} from './vim-motions';

export type VimMode = 'normal' | 'insert' | 'visual';
export type PendingKey = '' | 'g' | 'd' | 'c' | 'y' | 'z';
type NonEmptyPending = Exclude<PendingKey, ''>;
type Operator = 'd' | 'c' | 'y';

export const vim = $state<{ mode: VimMode; pending: PendingKey }>({ mode: 'normal', pending: '' });

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
  const { selection } = view.state;
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

function openLine(view: EditorView, below: boolean) {
  const { doc, schema, selection } = view.state;
  const fromPos = selection.$from;
  const inTitle = fromPos.parent.type.name === 'title';

  if (!below && inTitle) {
    moveCursor(view, fromPos.start(fromPos.depth));
    vim.mode = 'insert';
    vim.pending = '';
    return;
  }

  const insertPos = below
    ? inTitle
      ? doc.firstChild!.nodeSize
      : fromPos.after(1)
    : fromPos.before(1);
  const tr = view.state.tr.insert(insertPos, schema.nodes.paragraph.create());
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
  const isTrailingLastLine = fromPos.depth === 1 && fromPos.index(0) === doc.childCount - 1 && isEmptyLine(fromPos.parent);
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

function applyOperator(view: EditorView, op: Operator, from: number, to: number) {
  const { state } = view;
  if (to > from) {
    yank(state.doc.textBetween(from, to, '\n'), false);
    if (op !== 'y') {
      view.dispatch(state.tr.delete(from, to).scrollIntoView());
    } else {
      moveCursor(view, from);
    }
  } else if (op === 'y') {
    moveCursor(view, from);
  }
  vim.mode = op === 'c' ? 'insert' : 'normal';
  vim.pending = '';
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

interface ActionContext {
  editor: Editor;
  view: EditorView;
  pos: number;
  fromPos: ResolvedPos;
  end: number;
}

const PENDING_ACTIONS: Record<NonEmptyPending, Record<string, (ctx: ActionContext) => void>> = {
  g: {
    g: ({ view }) => moveCursor(view, topOfDoc(view.state)),
    c: () => toggleChat(),
    s: () => openCustomize(),
    w: () => {
      if (app.note) void updateMeta({ webOn: !app.note.webOn });
    },
    v: () => {
      if (app.note) void updateMeta({ voiceOn: !app.note.voiceOn });
    },
    m: () => {
      if (app.note) void updateMeta({ mode: app.note.mode === 'thinks' ? 'asks' : 'thinks' });
    },
  },
  d: {
    d: ({ view }) => deleteCurrentBlock(view),
    w: ({ view, pos }) => applyOperator(view, 'd', pos, nextWordStart(view.state.doc, pos, false)),
    $: ({ view, pos, end }) => applyOperator(view, 'd', pos, end),
  },
  c: {
    c: ({ view }) => changeCurrentBlock(view),
    w: ({ view, pos }) => applyOperator(view, 'c', pos, changeWordEnd(view.state.doc, pos)),
    $: ({ view, pos, end }) => applyOperator(view, 'c', pos, end),
  },
  y: {
    y: ({ fromPos }) => yank(parentText(fromPos), true),
  },
  z: {
    a: () => void toggleJustMyWords(),
  },
};

const NORMAL_ACTIONS: Record<string, (ctx: ActionContext) => void> = {
  i: () => {
    vim.mode = 'insert';
  },
  a: ({ view, pos, end }) => {
    moveCursor(view, Math.min(end, pos + 1));
    vim.mode = 'insert';
  },
  I: ({ view, fromPos }) => {
    moveCursor(view, firstNonSpacePos(fromPos));
    vim.mode = 'insert';
  },
  A: ({ view, end }) => {
    moveCursor(view, end);
    vim.mode = 'insert';
  },
  o: ({ view }) => openLine(view, true),
  O: ({ view }) => openLine(view, false),
  v: ({ view, pos }) => {
    vim.mode = 'visual';
    vim.pending = '';
    moveCursor(view, pos);
  },
  x: ({ view, pos, end }) => {
    if (pos < end) applyOperator(view, 'd', pos, pos + 1);
  },
  D: ({ view, pos, end }) => applyOperator(view, 'd', pos, end),
  C: ({ view, pos, end }) => applyOperator(view, 'c', pos, end),
  p: ({ view }) => pasteRegister(view, false),
  P: ({ view }) => pasteRegister(view, true),
  u: ({ editor }) => {
    editor.commands.undo();
  },
  '/': () => openFind(''),
  n: () => nextMatch({ moveCursor: true }),
  N: () => prevMatch({ moveCursor: true }),
};

const VISUAL_OPERATORS: Record<string, Operator> = {
  d: 'd',
  x: 'd',
  c: 'c',
  y: 'y',
};

function isPendingKey(key: string): key is NonEmptyPending {
  return key in PENDING_ACTIONS;
}

function consume(event: KeyboardEvent, run?: () => void): true {
  event.preventDefault();
  run?.();
  return true;
}

function actionContext(editor: Editor, view: EditorView): ActionContext {
  const { selection } = view.state;
  const fromPos = selection.$from;
  return { editor, view, pos: selection.from, fromPos, end: fromPos.end(fromPos.depth) };
}

function isSwallowedKey(key: string): boolean {
  return key.length === 1 || key === 'Backspace' || key === 'Delete' || key === 'Enter';
}

function handleVisualKey(view: EditorView, key: string, event: KeyboardEvent): boolean {
  if (vim.pending === 'g') {
    vim.pending = '';
    return consume(event, () => {
      if (key === 'g') extendVisual(view, topOfDoc(view.state));
    });
  }
  if (key === 'g') return consume(event, () => { vim.pending = 'g'; });
  if (key === 'v') return consume(event, () => exitToNormal(view));

  const motionTarget = resolveMotion(view, view.state.selection.head, key);
  if (motionTarget !== null) return consume(event, () => extendVisual(view, motionTarget));

  const op = VISUAL_OPERATORS[key];
  if (op) {
    const { from, to } = view.state.selection;
    return consume(event, () => applyOperator(view, op, from, to));
  }

  if (PASS_THROUGH_KEYS.has(key)) return false;
  if (isSwallowedKey(key)) return consume(event);
  return false;
}

function handleNormalKey(editor: Editor, view: EditorView, key: string, event: KeyboardEvent): boolean {
  if (vim.pending) {
    const pending = vim.pending;
    vim.pending = '';
    return consume(event, () => PENDING_ACTIONS[pending][key]?.(actionContext(editor, view)));
  }

  if (isPendingKey(key)) return consume(event, () => { vim.pending = key; });

  const action = NORMAL_ACTIONS[key];
  if (action) return consume(event, () => action(actionContext(editor, view)));

  const motionTarget = resolveMotion(view, view.state.selection.from, key);
  if (motionTarget !== null) return consume(event, () => moveCursor(view, motionTarget));

  if (PASS_THROUGH_KEYS.has(key)) return false;
  if (isSwallowedKey(key)) return consume(event, () => { vim.pending = ''; });
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
            if (anyInlineMenuOpen()) return false;

            const key = event.key;
            if (MODIFIER_KEYS.has(key)) return false;

            if (event.metaKey || event.ctrlKey || event.altKey) {
              if (event.ctrlKey && !event.metaKey && !event.altKey) {
                if (key === 'r' && vim.mode === 'normal') {
                  return consume(event, () => {
                    vim.pending = '';
                    editor.commands.redo();
                  });
                }
                if (key === '[' && (vim.mode === 'insert' || vim.mode === 'visual')) {
                  event.stopPropagation();
                  return consume(event, () => exitToNormal(view));
                }
              }
              return false;
            }

            if (key === 'Escape') {
              if (vim.mode === 'insert' || vim.mode === 'visual') {
                event.stopPropagation();
                return consume(event, () => exitToNormal(view));
              }
              if (vim.pending) {
                event.stopPropagation();
                return consume(event, () => { vim.pending = ''; });
              }
              if (!find.open && find.query) {
                event.stopPropagation();
                return consume(event, () => clearFindHighlights());
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

exposeDebug('vim', vim);
