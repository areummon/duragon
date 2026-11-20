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

const PACE = { regular: 900, slow: 1800 } as const;
const MIN_TARGET = 12;
/** New anchors fade their underline in while the pencil fades out. */
const FRESH_MS = 900;
/** Questions and requests are worth reading even when short: the read answers them inline. */
const MIN_REQUEST = 6;
const REQUEST = /^(?:draw|sketch|explain|show|tell|describe|compare|list|summari[sz]e|visuali[sz]e|what|why|how|who|when|where|can|could|should|is|are|do|does)\b/i;

const isRequest = (line: string) => line.endsWith('?') || REQUEST.test(line);

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
  const lastRead = new Set<string>();
  const answered = new Set<string>();
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
      untrack(() => view.dispatch(view.state.tr.setMeta(anchorKey, input)));
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

  async function answer(text: string) {
    answered.add(text);
    if (threads.list.some((t) => t.kind === 'reply' && t.quote === text)) return;
    await (await getBackend()).aiAnswer(noteId, text);
  }

  function cancelRead() {
    if (readTarget === null) return;
    const target = readTarget;
    readTarget = null;
    lastRead.delete(target);
    threads.thinking = threads.thinking.filter((t) => t.task !== 'read');
    void getBackend().then((be) => be.aiCancelRead(noteId));
  }

  return {
    delay: () => PACE[settings.value.readingPace],
    measure,
    onEdit() {
      collapseOnType();
      if (threads.thinking.some((t) => t.task === 'read')) cancelRead();
      else readTarget = null;
      const block = editor.state.selection.$from.index(0);
      if (block === caretBlock + 1 && !settings.value.justMyWords) {
        const line = editor.state.doc.child(caretBlock).textContent;
        if (isAddressed(line) && !answered.has(line)) void answer(line);
      }
      caretBlock = block;
    },
    onBlocksChanged(texts) {
      if (settings.value.justMyWords || !app.note) return;
      const lines = texts.map((t) => t.trim());
      for (const line of lines) if (isAddressed(line) && !answered.has(line)) void answer(line);
      if (app.note.mode !== 'thinks') return;
      const target = [...lines].reverse().find((t) => !isAddressed(t));
      if (!target || lastRead.has(target)) return;
      if (target.length < (isRequest(target) ? MIN_REQUEST : MIN_TARGET)) return;
      if (threads.list.some((t) => t.status === 'open' && target.includes(t.quote))) return;
      lastRead.add(target);
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
