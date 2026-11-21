import type { Editor } from '@tiptap/core';
import { mount, unmount, untrack } from 'svelte';
import { getBackend } from '../api/backend';
import InlineAnswer from '../components/InlineAnswer.svelte';
import ThreadCard from '../components/ThreadCard.svelte';
import { addressNames } from '../state/riddler';
import { layout, selectionUi, useMarks } from '../state/layout.svelte';
import { settings } from '../state/settings.svelte';
import { app } from '../state/app.svelte';
import { collapseOnType, geometry, open, threads, visibleThreads, type Geometry } from '../state/threads.svelte';
import { anchorKey, anchorPlugin, type AnchorInput } from './anchors';
import { measureLines } from './pencil';
import { current } from './current';
import { QUIET } from './create';
import { TextSelection } from '@tiptap/pm/state';
import type { Node as PMNode } from '@tiptap/pm/model';

const PACE = { regular: 900, slow: 1800 } as const;
const MIN_TARGET = 12;
/** New anchors fade their underline in while the pencil fades out. */
const FRESH_MS = 900;
/** Questions and requests are worth reading even when short: the read answers them inline. */
const MIN_REQUEST = 6;
/** An answer that arrived this recently gets a line opened below it; older ones, and answers loaded with the note, do not. */
const JUST_ARRIVED_MS = 4000;
/** Answers whose line has been opened, kept across re-attaching the editor so it happens once. */
const placed = new Set<string>();
const REQUEST = /^(?:draw|sketch|explain|show|tell|describe|compare|list|summari[sz]e|visuali[sz]e|what|why|how|who|when|where|can|could|should|is|are|do|does)\b/i;

const isRequest = (line: string) => line.endsWith('?') || REQUEST.test(line);

/** The note's lines below the title, trimmed, as the editor reports changed blocks. */
function bodyLines(doc: PMNode): string[] {
  const lines: string[] = [];
  doc.forEach((block, _offset, i) => {
    if (i > 0) lines.push(block.textBetween(0, block.content.size, '\n', '\n').trim());
  });
  return lines;
}

/**
 * How many copies of each line the mind has already been given. Counting copies, not remembering text, lets a line
 * that is pasted again, or deleted and written again, be read like any new line, while an unchanged line never is.
 */
function ledger() {
  const done = new Map<string, number>();
  return {
    pending: (line: string, copies: number) => (done.get(line) ?? 0) < copies,
    take: (line: string, copies: number) => done.set(line, copies),
    give: (line: string) => done.set(line, Math.max(0, (done.get(line) ?? 0) - 1)),
    /** Forget copies that are gone, so writing them again counts as new. */
    prune(lines: string[]) {
      for (const [line, n] of done) {
        const left = lines.filter((l) => l === line).length;
        if (left === 0) done.delete(line);
        else if (left < n) done.set(line, left);
      }
    },
  };
}

const copiesOf = (lines: string[], line: string) => Math.max(1, lines.filter((l) => l === line).length);

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

export function isAddressed(line: string): boolean {
  const names = addressNames().map(escape).join('|');
  return new RegExp(`^\\s*(?:hey\\s+)?(?:${names})\\s*[,:]`, 'i').test(line);
}

export interface Riddle {
  onEdit(): void;
  onBlocksChanged(texts: string[]): void;
  /** Debounce for read-along, from the reading pace. */
  delay(): number;
  measure(): void;
  detach(): void;
}

export function attachRiddle(editor: Editor, page: HTMLElement, noteId: string): Riddle {
  const view = editor.view;
  current.editor = editor;
  const read = ledger();
  const answered = ledger();
  let readTarget: string | null = null;
  let caretBlock = editor.state.selection.$from.index(0);
  let frame = 0;

  const plugin = anchorPlugin({
    onOpen: (id) => open(useMarks() && threads.openId === id ? null : id),
    onHover: (id) => (threads.hoverId = id),
    mountReply(id) {
      const dom = document.createElement('div');
      dom.className = 'inline-answer-host';
      dom.contentEditable = 'false';
      const c = mount(InlineAnswer, { target: dom, props: { id } });
      return { dom, destroy: () => void unmount(c) };
    },
    mountCard(id) {
      const dom = document.createElement('div');
      dom.className = 'thread-card-host';
      dom.contentEditable = 'false';
      const c = mount(ThreadCard, { target: dom, props: { id } });
      return { dom, destroy: () => void unmount(c) };
    },
  });
  editor.registerPlugin(plugin);

  function measure() {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      if (view.isDestroyed) return;
      const st = anchorKey.getState(view.state);
      if (!st) return;
      const box = page.getBoundingClientRect();
      const next: Record<string, Geometry | null> = {};
      for (const t of visibleThreads()) {
        const r = st.ranges.get(t.id);
        if (!r) {
          next[t.id] = null;
          continue;
        }
        const lines = measureLines(view, r, page);
        if (!lines.length) {
          next[t.id] = null;
          continue;
        }
        const first = lines[0];
        const last = lines[lines.length - 1];
        next[t.id] = {
          top: view.coordsAtPos(r.from, 1).top - box.top,
          startX: first.x1,
          startY: first.mid,
          endX: Math.max(...lines.map((l) => l.x2)),
          endY: last.mid,
        };
      }
      if (JSON.stringify(next) !== JSON.stringify(geometry.byId)) geometry.byId = next;
      geometry.pageWidth = box.width;
    });
  }

  /**
   * An answer sits below the line that asked for it. Writing continues below the answer: a request on the last line gets
   * an empty line after it, and a caret still resting at the end of the request moves down to that line.
   */
  function lineAfter(quoteEnd: number) {
    const { state } = view;
    const block = state.doc.resolve(quoteEnd - 1);
    const start = block.before(1);
    const end = block.after(1);
    const next = state.doc.nodeAt(end);
    const tr = state.tr;
    if (!next) tr.insert(end, state.schema.nodes.paragraph.create());
    const below = tr.doc.nodeAt(end);
    const sel = state.selection;
    const resting = sel.empty && sel.from > start && sel.from === end - 1;
    if (resting && below?.isTextblock && below.content.size === 0) tr.setSelection(TextSelection.create(tr.doc, end + 1));
    if (tr.docChanged || tr.selectionSet) view.dispatch(tr.setMeta('addToHistory', false).setMeta(QUIET, true));
  }

  const ro = new ResizeObserver(measure);
  ro.observe(view.dom);
  ro.observe(page);
  editor.on('update', measure);

  const stop = $effect.root(() => {
    $effect(() => {
      const input: AnchorInput = {
        threads: visibleThreads().map((t) => ({ id: t.id, quote: t.quote, kind: t.kind, status: t.status })),
        openId: threads.openId,
        hoverId: threads.hoverId,
        fresh: visibleThreads().filter((t) => performance.now() - (threads.arrivedAt[t.id] ?? -Infinity) < FRESH_MS).map((t) => t.id),
        marks: useMarks(),
        hidden: settings.value.justMyWords,
        pin: selectionUi.pin,
      };
      void layout.vw;
      if (view.isDestroyed) return;
      untrack(() => {
        view.dispatch(view.state.tr.setMeta(anchorKey, input));
        for (const t of input.threads) {
          const age = performance.now() - (threads.arrivedAt[t.id] ?? -Infinity);
          if (t.kind !== 'reply' || placed.has(t.id) || age > JUST_ARRIVED_MS) continue;
          placed.add(t.id);
          const r = anchorKey.getState(view.state)?.ranges.get(t.id);
          if (r) lineAfter(r.to);
        }
      });
      measure();
    });
  });

  const keydown = (e: KeyboardEvent) => {
    if (e.key !== 'Enter' || !(e.ctrlKey || e.metaKey)) return;
    const { from, to, empty } = editor.state.selection;
    if (empty) return;
    e.preventDefault();
    const text = editor.state.doc.textBetween(from, to, '\n');
    void getBackend().then((be) => be.aiAsk(noteId, text, null));
  };
  view.dom.addEventListener('keydown', keydown);

  async function answer(text: string, copies: number) {
    answered.take(text, copies);
    if (threads.list.filter((t) => t.kind === 'reply' && t.quote === text).length >= copies) return;
    await (await getBackend()).aiAnswer(noteId, text);
  }

  function cancelRead() {
    if (readTarget === null) return;
    const target = readTarget;
    readTarget = null;
    read.give(target);
    threads.thinking = threads.thinking.filter((t) => t.task !== 'read');
    void getBackend().then((be) => be.aiCancelRead(noteId));
  }

  return {
    delay: () => PACE[settings.value.readingPace],
    measure,
    onEdit() {
      collapseOnType();
      // A read stays valid while its line is unchanged: pressing Enter after it, or writing elsewhere, keeps it.
      // Runs on every keystroke, so it reads the doc directly rather than serializing it.
      if (readTarget !== null && !bodyLines(editor.state.doc).includes(readTarget)) cancelRead();
      const block = editor.state.selection.$from.index(0);
      if (block === caretBlock + 1 && !settings.value.justMyWords) {
        const line = editor.state.doc.child(caretBlock).textContent.trim();
        if (isAddressed(line)) {
          const copies = copiesOf(bodyLines(editor.state.doc), line);
          if (answered.pending(line, copies)) void answer(line, copies);
        }
      }
      caretBlock = block;
    },
    onBlocksChanged(texts) {
      if (settings.value.justMyWords || !app.note) return;
      const all = bodyLines(editor.state.doc);
      read.prune(all);
      answered.prune(all);
      const lines = texts.map((t) => t.trim());
      for (const line of lines) {
        const copies = copiesOf(all, line);
        if (isAddressed(line) && answered.pending(line, copies)) void answer(line, copies);
      }
      if (app.note.mode !== 'thinks') return;
      const target = [...lines].reverse().find((t) => !isAddressed(t));
      if (!target) return;
      const copies = copiesOf(all, target);
      if (!read.pending(target, copies)) return;
      if (target.length < (isRequest(target) ? MIN_REQUEST : MIN_TARGET)) return;
      // Each open thread on these words covers one copy of the line; a copy beyond them is new.
      if (threads.list.filter((t) => t.status === 'open' && t.quote && target.includes(t.quote)).length >= copies) return;
      read.take(target, copies);
      readTarget = target;
      void getBackend().then((be) => be.aiRead(noteId, target));
    },
    detach() {
      stop();
      ro.disconnect();
      cancelAnimationFrame(frame);
      view.dom.removeEventListener('keydown', keydown);
      editor.off('update', measure);
      if (!view.isDestroyed) editor.unregisterPlugin(anchorKey);
      if (current.editor === editor) current.editor = null;
    },
  };
}
