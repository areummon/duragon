import { Extension, type Editor } from '@tiptap/core';
import type { Node as PMNode } from '@tiptap/pm/model';
import { Plugin, PluginKey, TextSelection } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';

export interface FindMatch {
  from: number;
  to: number;
}

export const FIND_META = 'duragon-find';

export const find = $state({
  open: false,
  query: '',
  matches: [] as FindMatch[],
  index: 0,
});

/** Incremented whenever the find input should grab focus and select its contents. */
export const findFocus = $state({ seq: 0 });

const findPluginKey = new PluginKey<DecorationSet>('findHighlight');

let activeEditor: Editor | null = null;
let lastQuery = '';

interface TextSegment {
  text: string;
  pos: number;
}

function searchRun(run: TextSegment[], needle: string, out: FindMatch[]) {
  if (run.length === 0) return;
  let full = '';
  const starts: number[] = [];
  for (const seg of run) {
    starts.push(full.length);
    full += seg.text;
  }

  const lower = full.toLowerCase();
  const len = needle.length;
  let offset = 0;

  while (offset <= lower.length - len) {
    const hit = lower.indexOf(needle, offset);
    if (hit === -1) break;
    const from = mapStart(run, starts, hit);
    const to = mapEnd(run, starts, hit + len);
    if (from !== null && to !== null && to > from) {
      out.push({ from, to });
    }
    offset = hit + len;
  }
}

function mapStart(run: TextSegment[], starts: number[], idx: number): number | null {
  for (let i = run.length - 1; i >= 0; i--) {
    if (idx >= starts[i]) {
      const local = Math.min(idx - starts[i], run[i].text.length);
      return run[i].pos + local;
    }
  }
  return null;
}

function mapEnd(run: TextSegment[], starts: number[], idx: number): number | null {
  for (let i = run.length - 1; i >= 0; i--) {
    if (idx > starts[i]) {
      const local = Math.min(idx - starts[i], run[i].text.length);
      return run[i].pos + local;
    }
  }
  return null;
}

function computeMatches(doc: PMNode, query: string): FindMatch[] {
  if (!query) return [];
  const needle = query.toLowerCase();
  const out: FindMatch[] = [];

  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    let run: TextSegment[] = [];
    node.descendants((child, childPos) => {
      if (child.isText && child.text) {
        run.push({ text: child.text, pos: pos + 1 + childPos });
      } else if (child.isLeaf) {
        searchRun(run, needle, out);
        run = [];
      }
      return true;
    });
    searchRun(run, needle, out);
    return false;
  });

  return out;
}

function buildDecorations(doc: PMNode): DecorationSet {
  if (!find.query) {
    if (find.matches.length > 0) find.matches = [];
    find.index = 0;
    return DecorationSet.empty;
  }

  const matches = computeMatches(doc, find.query);
  find.matches = matches;
  if (matches.length === 0) {
    find.index = 0;
    return DecorationSet.empty;
  }
  if (find.index >= matches.length) find.index = matches.length - 1;
  if (find.index < 0) find.index = 0;

  const decos = matches.map((m, i) =>
    Decoration.inline(m.from, m.to, {
      class: i === find.index ? 'find-match find-match-current' : 'find-match',
    }),
  );
  return DecorationSet.create(doc, decos);
}

function scrollCurrentIntoView(editor: Editor) {
  const el = editor.view.dom.querySelector('.find-match-current');
  if (!(el instanceof HTMLElement)) return;
  const rect = el.getBoundingClientRect();
  if (rect.top >= 80 && rect.bottom <= window.innerHeight - 40) return;
  const behavior = window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth';
  el.scrollIntoView({ block: 'center', behavior });
}

function refreshEditor(opts: { scroll?: boolean; moveCursor?: boolean; focus?: boolean } = {}) {
  const ed = activeEditor;
  if (!ed || ed.isDestroyed) return;

  const { state, view } = ed;
  let tr = state.tr.setMeta(FIND_META, true);

  if (opts.moveCursor && find.matches.length > 0) {
    const match = find.matches[find.index];
    if (match) {
      const clamped = Math.max(0, Math.min(match.from, state.doc.content.size));
      tr = tr.setSelection(TextSelection.create(state.doc, clamped)).scrollIntoView();
    }
  }

  view.dispatch(tr);

  if (opts.focus) {
    view.focus();
  }
  if (opts.scroll && !opts.moveCursor) {
    scrollCurrentIntoView(ed);
  }
}

function firstIndexNearSelection(matches: FindMatch[], editor: Editor | null): number {
  if (matches.length === 0 || !editor || editor.isDestroyed) return 0;
  const pos = editor.state.selection.from;
  const after = matches.findIndex((m) => m.from >= pos);
  return after >= 0 ? after : 0;
}

export function attachFindEditor(editor: Editor | null) {
  activeEditor = editor;
  find.open = false;
  find.query = '';
  find.matches = [];
  find.index = 0;
  lastQuery = '';
}

export function openFind(initialQuery?: string) {
  if (initialQuery !== undefined) {
    find.query = initialQuery;
    if (initialQuery) lastQuery = initialQuery;
  } else if (!find.open && !find.query && activeEditor && !activeEditor.isDestroyed) {
    const { state } = activeEditor;
    const { from, to, empty } = state.selection;
    if (!empty && to - from <= 120) {
      const selected = state.doc.textBetween(from, to, '\n');
      if (selected && !selected.includes('\n')) {
        find.query = selected;
        lastQuery = selected;
      }
    }
  }

  find.open = true;
  findFocus.seq += 1;

  if (activeEditor && !activeEditor.isDestroyed) {
    const matches = computeMatches(activeEditor.state.doc, find.query);
    find.matches = matches;
    find.index = firstIndexNearSelection(matches, activeEditor);
    refreshEditor({ scroll: matches.length > 0 });
  }
}

export function setFindQuery(q: string) {
  find.query = q;
  if (q) lastQuery = q;

  if (activeEditor && !activeEditor.isDestroyed) {
    const matches = computeMatches(activeEditor.state.doc, q);
    find.matches = matches;
    find.index = firstIndexNearSelection(matches, activeEditor);
    refreshEditor({ scroll: matches.length > 0 });
  } else {
    find.matches = [];
    find.index = 0;
  }
}

export function nextMatch(opts: { moveCursor?: boolean } = {}) {
  const ed = activeEditor;
  if (!ed || ed.isDestroyed) return;

  if (!find.query) {
    if (!lastQuery) return;
    find.query = lastQuery;
    const matches = computeMatches(ed.state.doc, find.query);
    find.matches = matches;
    if (matches.length === 0) return;
    const pos = ed.state.selection.from;
    const after = matches.findIndex((m) => m.from > pos);
    find.index = after >= 0 ? after : 0;
    refreshEditor({ scroll: true, moveCursor: opts.moveCursor });
    return;
  }

  const matches = computeMatches(ed.state.doc, find.query);
  find.matches = matches;
  if (matches.length === 0) {
    find.index = 0;
    refreshEditor();
    return;
  }

  find.index = (find.index + 1) % matches.length;
  refreshEditor({ scroll: true, moveCursor: opts.moveCursor });
}

export function prevMatch(opts: { moveCursor?: boolean } = {}) {
  const ed = activeEditor;
  if (!ed || ed.isDestroyed) return;

  if (!find.query) {
    if (!lastQuery) return;
    find.query = lastQuery;
    const matches = computeMatches(ed.state.doc, find.query);
    find.matches = matches;
    if (matches.length === 0) return;
    const pos = ed.state.selection.from;
    let before = -1;
    for (let i = matches.length - 1; i >= 0; i--) {
      if (matches[i].from < pos) {
        before = i;
        break;
      }
    }
    find.index = before >= 0 ? before : matches.length - 1;
    refreshEditor({ scroll: true, moveCursor: opts.moveCursor });
    return;
  }

  const matches = computeMatches(ed.state.doc, find.query);
  find.matches = matches;
  if (matches.length === 0) {
    find.index = 0;
    refreshEditor();
    return;
  }

  find.index = (find.index - 1 + matches.length) % matches.length;
  refreshEditor({ scroll: true, moveCursor: opts.moveCursor });
}

/** Confirms the active search (used by Enter in Vim mode): closes the bar, focuses the editor at the current match. */
export function confirmFind() {
  if (find.query) lastQuery = find.query;
  find.open = false;
  refreshEditor({ moveCursor: find.matches.length > 0, focus: true });
}

/** Clears active highlights while keeping `lastQuery` for Vim `n` / `N`. */
export function clearFindHighlights() {
  if (find.query) lastQuery = find.query;
  find.query = '';
  find.matches = [];
  find.index = 0;
  refreshEditor();
}

export function closeFind(opts: { focusEditor?: boolean } = {}) {
  if (find.query) lastQuery = find.query;
  find.open = false;
  find.query = '';
  find.matches = [];
  find.index = 0;
  refreshEditor({ focus: opts.focusEditor });
}

export const FindHighlight = Extension.create({
  name: 'findHighlight',

  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key: findPluginKey,
        state: {
          init(_config, state) {
            return buildDecorations(state.doc);
          },
          apply(tr, oldDecos, _oldState, newState) {
            if (!tr.docChanged && !tr.getMeta(FIND_META)) return oldDecos;
            return buildDecorations(newState.doc);
          },
        },
        props: {
          decorations(state) {
            return findPluginKey.getState(state);
          },
        },
      }),
    ];
  },
});

if (import.meta.env.DEV) {
  const win = window as unknown as { __duragon?: Record<string, unknown> };
  win.__duragon ??= {};
  win.__duragon.find = find;
}
