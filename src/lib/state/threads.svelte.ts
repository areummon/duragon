import { getBackend } from '../api/backend';
import type { AiTask, Message, Thread, ThreadStatus } from '../api/types';
import { app, openNote } from './app.svelte';
import { upsertMeta } from './notes.svelte';

export interface Thinking {
  id: number;
  task: AiTask;
  target: string;
}

export interface Geometry {
  /** Top of the quote's first line, relative to the page. */
  top: number;
  /** Where the quote starts: left of its first line, y of that line's middle. */
  startX: number;
  startY: number;
  /** Where the quote ends: right of its widest line, y of its last line's middle. */
  endX: number;
  endY: number;
}

export const threads = $state({
  noteId: null as string | null,
  list: [] as Thread[],
  thinking: [] as Thinking[],
  /** Threads waiting for Riddled's answer to a reply. */
  replying: {} as Record<string, boolean>,
  /** Messages that arrived this session. Only these animate in. */
  freshText: {} as Record<string, true>,
  freshDraw: {} as Record<string, true>,
  openId: null as string | null,
  hoverId: null as string | null,
  fullId: null as string | null,
  /** Reads that came back with nothing to add, until their mark has played. */
  quiet: [] as { id: number; target: string }[],
  /** When each thread arrived this session (performance.now()), for its entrance. */
  arrivedAt: {} as Record<string, number>,
});

/** Quote positions in the page, filled by the editor glue. null means the quote is gone. */
export const geometry = $state<{ byId: Record<string, Geometry | null>; pageWidth: number }>({ byId: {}, pageWidth: 690 });

export const ASKING: AiTask[] = ['read', 'answer', 'ask', 'visualize'];
let unsubscribe: (() => void)[] = [];
let seq = 0;

const isLocal = (id: string) => id.startsWith('local-');

function mergeMessage(t: Thread, m: Message) {
  const i = t.messages.findIndex((x) => x.id === m.id || (isLocal(x.id) && x.author === m.author && x.text === m.text));
  if (i >= 0) t.messages[i] = m;
  else t.messages.push(m);
}

/** Freshness plays the arrival once (ink-in, blot splat, drawing strokes); a view mounted later shows it settled. */
const INK_DRY_MS = 3400;
const INK_WORD_MS = 19;
const DRAW_MS = 8000;

function expire(set: Record<string, true>, id: string, ms: number) {
  window.setTimeout(() => delete set[id], ms);
}

function noteMessage(m: Message, before: Message | undefined) {
  if (m.author !== 'riddled') return;
  if (!before) {
    threads.freshText[m.id] = true;
    expire(threads.freshText, m.id, Math.max(INK_DRY_MS, m.text.split(/\s+/).length * INK_WORD_MS + 640) + 200);
  }
  if (m.drawing === 'done' && before?.drawing !== 'done') {
    threads.freshDraw[m.id] = true;
    expire(threads.freshDraw, m.id, DRAW_MS);
  }
}

function dropThinking(match: (t: Thinking) => boolean) {
  const i = threads.thinking.findIndex(match);
  if (i >= 0) threads.thinking.splice(i, 1);
}

export async function loadThreads(noteId: string) {
  unloadThreads();
  const be = await getBackend();
  threads.noteId = noteId;
  threads.openId = threads.hoverId = threads.fullId = null;
  threads.thinking = [];
  threads.quiet = [];
  threads.arrivedAt = {};
  threads.replying = {};
  threads.freshText = {};
  threads.freshDraw = {};
  threads.list = await be.listThreads(noteId);
  if (threads.noteId !== noteId) return;

  unsubscribe = [
    be.on('ai:thinking', (p) => {
      if (p.noteId === noteId) threads.thinking.push({ id: ++seq, task: p.task, target: p.target });
    }),
    be.on('ai:thread', (t) => {
      if (t.noteId !== noteId) return;
      const i = threads.list.findIndex((x) => x.id === t.id);
      const before = i >= 0 ? threads.list[i] : undefined;
      for (const m of t.messages) noteMessage(m, before?.messages.find((x) => x.id === m.id));
      if (i >= 0) threads.list[i] = t;
      else {
        threads.list.push(t);
        threads.arrivedAt[t.id] = performance.now();
      }
      dropThinking((x) => ASKING.includes(x.task));
      refreshOpenFlag();
    }),
    be.on('ai:message', (m) => {
      if (m.noteId !== noteId || !m.threadId) return;
      const t = threads.list.find((x) => x.id === m.threadId);
      if (!t) return;
      noteMessage(m, t.messages.find((x) => x.id === m.id));
      mergeMessage(t, m);
      if (m.author === 'riddled') {
        delete threads.replying[t.id];
        dropThinking((x) => x.task === 'reply');
      }
    }),
    be.on('ai:quiet', (p) => {
      if (p.noteId !== noteId) return;
      dropThinking((x) => x.target === p.target);
      threads.quiet.push({ id: ++seq, target: p.target });
    }),
    be.on('ai:error', (p) => {
      if (p.noteId !== noteId) return;
      dropThinking((x) => x.task === p.task);
      if (p.task === 'reply') threads.replying = {};
    }),
  ];
}

export function unloadThreads() {
  unsubscribe.forEach((u) => u());
  unsubscribe = [];
  threads.noteId = null;
  threads.list = [];
}

function refreshOpenFlag() {
  const open = threads.list.some((t) => t.status === 'open');
  if (app.note) upsertMeta({ ...$state.snapshot(app.note), hasOpenQuestion: open });
}

export const find = (id: string | null) => (id ? threads.list.find((t) => t.id === id) : undefined);
export const visibleThreads = () => threads.list.filter((t) => t.status === 'open');
export const openCount = () => visibleThreads().length;

export function open(id: string | null) {
  threads.openId = id;
}

/** Typing in the editor folds open threads unless they are pinned. */
export function collapseOnType() {
  const t = find(threads.openId);
  if (t && !t.pinned) threads.openId = null;
}

export async function setStatus(id: string, status: ThreadStatus) {
  const t = find(id);
  if (!t) return;
  t.status = status;
  if (threads.openId === id) threads.openId = null;
  if (threads.fullId === id) threads.fullId = null;
  await (await getBackend()).setThreadStatus(id, status);
  refreshOpenFlag();
}

export async function togglePin(id: string) {
  const t = find(id);
  if (!t) return;
  t.pinned = !t.pinned;
  await (await getBackend()).setThreadPinned(id, t.pinned);
}

/** An answer under its line becomes a margin question about that line. */
export async function moveToMargin(id: string) {
  const t = find(id);
  if (!t) return;
  t.kind = 'question';
  if (threads.openId === id) threads.openId = null;
  await (await getBackend()).setThreadKind(id, 'question');
}

export async function reply(id: string, text: string) {
  const t = find(id);
  if (!t || !text.trim()) return;
  t.messages.push({
    id: `local-${++seq}`, noteId: t.noteId, threadId: id, author: 'you', text, drawing: 'none', svg: null, sources: [], symbols: [], trimmed: false, createdAt: Date.now(),
  });
  threads.replying[id] = true;
  await (await getBackend()).aiReply(id, text);
}

export async function makeNote(id: string) {
  const note = await (await getBackend()).threadToNote(id);
  upsertMeta(note);
  await openNote(note.id);
}

export function threadText(t: Thread): string {
  return t.messages.map((m) => `${m.author === 'you' ? 'You' : 'Riddled'}: ${m.text}`).join('\n\n');
}

if (import.meta.env.DEV) Object.assign((window as unknown as { __duragon: object }).__duragon ?? {}, { threads });
