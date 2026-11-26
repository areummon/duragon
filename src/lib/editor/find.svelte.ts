import { Extension, type Editor } from '@tiptap/core';
import type { Node as PMNode } from '@tiptap/pm/model';
import { Plugin, PluginKey, TextSelection } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { exposeDebug } from '../debug';
import { settings } from '../state/settings.svelte';
import { current } from './current';

export interface FindMatch {
  from: number;
  to: number;
}

interface FindPluginState {
  decos: DecorationSet;
  matches: FindMatch[];
  index: number;
}

interface FindMetaPayload {
  matches: FindMatch[];
  index: number;
}

export const FIND_META = 'duragon-find';
const EMPTY_MATCHES: FindMatch[] = [];

export const find = $state({
  open: false,
  query: '',
  matches: EMPTY_MATCHES as FindMatch[],
  index: 0,
});

/** Incremented whenever the find input should grab focus and select its contents. */
export const findFocus = $state({ seq: 0 });

const findPluginKey = new PluginKey<FindPluginState>('findHighlight');

let lastQuery = '';

function activeEditor(): Editor | null {
  const ed = current.editor;
  return ed && !ed.isDestroyed ? ed : null;
}

interface TextSegment {
  text: string;
  pos: number;
}

function mapOffset(run: TextSegment[], starts: number[], idx: number, isEnd: boolean): number | null {
  for (let i = run.length - 1; i >= 0; i--) {
    if (isEnd ? idx > starts[i] : idx >= starts[i]) {
      const local = Math.min(idx - starts[i], run[i].text.length);
      return run[i].pos + local;
    }
  }
  return null;
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
    const from = mapOffset(run, starts, hit, false);
    const to = mapOffset(run, starts, hit + len, true);
    if (from !== null && to !== null && to > from) {
      out.push({ from, to });
    }
    offset = hit + len;
  }
}

function computeMatches(doc: PMNode, query: string): FindMatch[] {
  if (!query) return EMPTY_MATCHES;
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

  return out.length > 0 ? out : EMPTY_MATCHES;
}

function clampIndex(length: number, index: number): number {
  if (length === 0) return 0;
  return Math.max(0, Math.min(index, length - 1));
}

function makeDecorations(doc: PMNode, matches: FindMatch[], index: number): DecorationSet {
  if (matches.length === 0) return DecorationSet.empty;
  const decos = matches.map((m, i) =>
    Decoration.inline(m.from, m.to, {
      class: i === index ? 'find-match find-match-current' : 'find-match',
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

function dispatchFind(opts: { scroll?: boolean; moveCursor?: boolean; focus?: boolean } = {}) {
  const ed = activeEditor();
  if (!ed) return;

  const { state, view } = ed;
  const payload: FindMetaPayload = { matches: find.matches, index: find.index };
  let tr = state.tr.setMeta(FIND_META, payload);

  if (opts.moveCursor && find.matches.length > 0) {
    const match = find.matches[find.index];
    if (match) {
      const clamped = Math.max(0, Math.min(match.from, state.doc.content.size));
      tr = tr.setSelection(TextSelection.create(state.doc, clamped)).scrollIntoView();
    }
  }

  view.dispatch(tr);

  if (opts.focus) view.focus();
  if (opts.scroll && !opts.moveCursor) scrollCurrentIntoView(ed);
}

function firstIndexNearSelection(matches: FindMatch[], editor: Editor): number {
  if (matches.length === 0) return 0;
  const pos = editor.state.selection.from;
  const after = matches.findIndex((m) => m.from >= pos);
  return after >= 0 ? after : 0;
}

function findInitialStepIndex(matches: FindMatch[], pos: number, dir: 1 | -1): number {
  if (dir > 0) {
    const after = matches.findIndex((m) => m.from > pos);
    return after >= 0 ? after : 0;
  }
  for (let i = matches.length - 1; i >= 0; i--) {
    if (matches[i].from < pos) return i;
  }
  return matches.length - 1;
}

export function resetFind() {
  find.open = false;
  find.query = '';
  find.matches = EMPTY_MATCHES;
  find.index = 0;
  lastQuery = '';
}

export function openFind(initialQuery?: string) {
  const ed = activeEditor();
  if (initialQuery !== undefined) {
    find.query = initialQuery;
    if (initialQuery) lastQuery = initialQuery;
  } else if (!find.open && !find.query && ed) {
    const { state } = ed;
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

  if (ed) {
    const matches = computeMatches(ed.state.doc, find.query);
    find.matches = matches;
    find.index = firstIndexNearSelection(matches, ed);
    dispatchFind({ scroll: matches.length > 0 });
  }
}

export function setFindQuery(q: string) {
  find.query = q;
  if (q) lastQuery = q;

  const ed = activeEditor();
  if (ed) {
    const matches = computeMatches(ed.state.doc, q);
    find.matches = matches;
    find.index = firstIndexNearSelection(matches, ed);
    dispatchFind({ scroll: matches.length > 0 });
  } else {
    find.matches = EMPTY_MATCHES;
    find.index = 0;
  }
}

export function stepMatch(dir: 1 | -1, opts: { moveCursor?: boolean } = {}) {
  const ed = activeEditor();
  if (!ed) return;

  const reviving = !find.query;
  if (reviving) {
    if (!lastQuery) return;
    find.query = lastQuery;
  }

  const matches = computeMatches(ed.state.doc, find.query);
  find.matches = matches;
  if (matches.length === 0) {
    find.index = 0;
    if (!reviving) dispatchFind();
    return;
  }

  find.index = reviving
    ? findInitialStepIndex(matches, ed.state.selection.from, dir)
    : (find.index + dir + matches.length) % matches.length;
  dispatchFind({ scroll: true, moveCursor: opts.moveCursor });
}

export const nextMatch = (opts?: { moveCursor?: boolean }) => stepMatch(1, opts);
export const prevMatch = (opts?: { moveCursor?: boolean }) => stepMatch(-1, opts);

/** Confirms the active search (used by Enter in Vim mode): closes the bar, focuses the editor at the current match. */
export function confirmFind() {
  if (find.query) lastQuery = find.query;
  find.open = false;
  dispatchFind({ moveCursor: find.matches.length > 0, focus: true });
}

/** Handles Enter / Shift+Enter inside the find bar, encapsulating Vim vs normal confirm semantics. */
export function submitFind(opts: { backwards?: boolean } = {}) {
  if (opts.backwards) prevMatch();
  else if (settings.value.vimMode) confirmFind();
  else nextMatch();
}

/** Clears active highlights while keeping `lastQuery` for Vim `n` / `N`. */
export function clearFindHighlights() {
  if (find.query) lastQuery = find.query;
  find.query = '';
  find.matches = EMPTY_MATCHES;
  find.index = 0;
  dispatchFind();
}

export function closeFind(opts: { focusEditor?: boolean } = {}) {
  if (find.query) lastQuery = find.query;
  find.open = false;
  find.query = '';
  find.matches = EMPTY_MATCHES;
  find.index = 0;
  dispatchFind({ focus: opts.focusEditor });
}

export const FindHighlight = Extension.create({
  name: 'findHighlight',

  addProseMirrorPlugins() {
    return [
      new Plugin<FindPluginState>({
        key: findPluginKey,
        state: {
          init(_config, state) {
            const matches = computeMatches(state.doc, find.query);
            const index = clampIndex(matches.length, find.index);
            return { decos: makeDecorations(state.doc, matches, index), matches, index };
          },
          apply(tr, prev, _oldState, newState) {
            const meta = tr.getMeta(FIND_META) as FindMetaPayload | undefined;
            if (meta) {
              const index = clampIndex(meta.matches.length, meta.index);
              return {
                decos: makeDecorations(newState.doc, meta.matches, index),
                matches: meta.matches,
                index,
              };
            }
            if (!tr.docChanged) return prev;
            const matches = computeMatches(newState.doc, find.query);
            const index = clampIndex(matches.length, prev.index);
            return { decos: makeDecorations(newState.doc, matches, index), matches, index };
          },
        },
        view: () => ({
          update(view) {
            const ps = findPluginKey.getState(view.state);
            if (!ps) return;
            if (find.matches !== ps.matches) find.matches = ps.matches;
            if (find.index !== ps.index) find.index = ps.index;
          },
        }),
        props: {
          decorations(state) {
            return findPluginKey.getState(state)?.decos;
          },
        },
      }),
    ];
  },
});

exposeDebug('find', find);
