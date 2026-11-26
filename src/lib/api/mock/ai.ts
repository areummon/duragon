import { CHAT_BUDGET, SUMMARIZE_AFTER, SUMMARIZE_FOLD, TASK_BUDGET } from '../limits';
import type { AiTask, BackendEvents, Message, Note, Settings, Thread } from '../types';
import {
  CHAT_ANSWER,
  DRAW,
  QUESTIONS,
  REQUEST,
  RICH,
  RICH_ANSWER,
  RICH_SYMBOLS,
  SOURCES,
  drawingFor,
  hash,
  now,
  plainOf,
  uid,
} from './fixtures';

export interface MockAiContext {
  notes: Map<string, Note>;
  threads: Map<string, Thread>;
  chat: Map<string, Message[]>;
  summaries: Map<string, { text: string; through: string | null }>;
  getSettings: () => Settings;
  emit: <K extends keyof BackendEvents>(event: K, payload: BackendEvents[K]) => void;
}

export class MockAi {
  private timers = new Map<string, number[]>();

  constructor(private ctx: MockAiContext) {}

  private schedule(key: string, ms: number, fn: () => void) {
    const id = window.setTimeout(fn, ms);
    this.timers.set(key, [...(this.timers.get(key) ?? []), id]);
  }

  cancel(key: string) {
    this.timers.get(key)?.forEach(clearTimeout);
    this.timers.delete(key);
  }

  private msg(noteId: string, threadId: string | null, text: string, drawing: Message['drawing'] = 'none'): Message {
    const note = this.ctx.notes.get(noteId);
    const web = note?.webOn ?? false;
    const sources = web ? SOURCES.slice(0, 1 + (hash(text) % 2)) : [];
    const symbols = this.ctx.getSettings().explainSymbols && text === RICH_ANSWER ? RICH_SYMBOLS : [];
    const budget = threadId === null ? CHAT_BUDGET : TASK_BUDGET;
    const trimmed = !!note && Math.ceil(plainOf(note.doc).length / 4) > budget;
    return { id: uid(), noteId, threadId, author: 'riddled', text, drawing, svg: null, sources, symbols, trimmed, createdAt: now() };
  }

  private think(noteId: string, target: string, task: AiTask) {
    this.ctx.emit('ai:thinking', { noteId, target, task });
  }

  private wait() {
    return 1200 + Math.random() * 1300;
  }

  /** Drawing arrives later: the message is emitted as pending, then as done with the svg. */
  private finishDrawing(key: string, m: Message, topic: string) {
    this.schedule(key, 1800 + Math.random() * 700, () => {
      m.drawing = 'done';
      m.svg = drawingFor(topic);
      this.ctx.emit('ai:message', { ...m });
    });
  }

  private openThread(noteId: string, kind: Thread['kind'], quote: string, text: string, wantsDrawing: boolean, topic: string) {
    if (!this.ctx.notes.has(noteId)) return;
    const th: Thread = { id: uid(), noteId, kind, quote, status: 'open', pinned: false, createdAt: now(), messages: [] };
    const m = this.msg(noteId, th.id, text, wantsDrawing ? 'pending' : 'none');
    th.messages.push(m);
    this.ctx.threads.set(th.id, th);
    this.ctx.emit('ai:thread', structuredClone(th));
    if (wantsDrawing) this.finishDrawing(th.id, m, topic);
  }

  private firstSentence(target: string) {
    const s = target.match(/[^.!?]+[.!?]?/)?.[0].trim() ?? target;
    return (s.length >= 12 ? s : target).slice(0, 90).trim();
  }

  private answerText(topic: string) {
    if (RICH.test(topic)) return RICH_ANSWER;
    if (/segment\s*tree/i.test(topic)) {
      return 'Show a segment tree over [2, 5, 1, 4] with leaf values at the bottom, pair sums 7 and 5 in the middle, and total 12 at the root. Range queries and point updates touch at most two nodes per level, keeping both O(log n).';
    }
    return `Here is how I would put it: ${topic.replace(/[.?!]+$/, '')} comes down to a few moving parts that depend on each other. Tell me where that does not fit what you had in mind.`;
  }

  /** Like Ai::summarize. */
  private foldChat(noteId: string) {
    const list = this.ctx.chat.get(noteId) ?? [];
    const prev = this.ctx.summaries.get(noteId) ?? { text: '', through: null };
    const pending = list.slice(list.findIndex((m) => m.id === prev.through) + 1);
    if (pending.length <= SUMMARIZE_AFTER) return;
    const fold = pending.slice(0, SUMMARIZE_FOLD);
    const asked = fold.filter((m) => m.author === 'you').map((m) => `“${m.text}”`).join(', ');
    const text = `${prev.text ? `${prev.text} ` : ''}The writer asked ${asked}, and the replies pointed at the claim that the whole process happens in one place.`;
    this.ctx.summaries.set(noteId, { text, through: fold[fold.length - 1].id });
    this.ctx.emit('chat:summary', { noteId, summary: text });
  }

  /** Like the real read pass: questions and requests get an inline reply, other prose a margin question or nothing. */
  async aiRead(noteId: string, target: string) {
    this.think(noteId, target, 'read');
    this.schedule(noteId, 1500 + Math.random() * 1500, () => {
      if (target.endsWith('?') || REQUEST.test(target)) return this.openThread(noteId, 'reply', target, this.answerText(target), DRAW.test(target), target);
      if (hash(target) % 4 === 0) return this.ctx.emit('ai:quiet', { noteId, target });
      const q = RICH.test(target) ? RICH_ANSWER : QUESTIONS[hash(target) % QUESTIONS.length];
      this.openThread(noteId, 'question', this.firstSentence(target), q, DRAW.test(target), target);
    });
  }

  async aiAnswer(noteId: string, target: string) {
    this.think(noteId, target, 'answer');
    this.schedule(noteId, this.wait(), () => {
      const topic = target.replace(/^\s*(hey\s+)?\w+\s*[,:]\s*/i, '');
      this.openThread(noteId, 'reply', target, this.answerText(topic), DRAW.test(target), topic);
    });
  }

  async aiAsk(noteId: string, target: string, question: string | null) {
    this.think(noteId, target, 'ask');
    this.schedule(noteId, this.wait(), () => {
      const text = question
        ? `On your question, “${question}”: I would start from what this passage assumes, then check each assumption against a case where it fails.`
        : QUESTIONS[hash(target) % QUESTIONS.length];
      this.openThread(noteId, 'question', target, text, DRAW.test(target + (question ?? '')), target);
    });
  }

  async aiVisualize(noteId: string, target: string) {
    this.think(noteId, target, 'visualize');
    this.schedule(noteId, this.wait(), () => {
      this.openThread(noteId, 'question', target, 'Here is that drawn out, one step at a time.', true, target);
    });
  }

  async aiReply(threadId: string, text: string) {
    const th = this.ctx.threads.get(threadId);
    if (!th) throw new Error(`thread not found: ${threadId}`);
    th.messages.push({ id: uid(), noteId: th.noteId, threadId, author: 'you', text, drawing: 'none', svg: null, sources: [], symbols: [], trimmed: false, createdAt: now() });
    this.think(th.noteId, text, 'reply');
    this.schedule(threadId, this.wait(), () => {
      const draws = DRAW.test(text);
      const m = this.msg(
        th.noteId,
        threadId,
        draws ? 'Let me show that as a picture. Watch the order of the steps, because the order is the point.' : RICH.test(text) ? RICH_ANSWER : 'That makes sense. What would change your mind?',
        draws ? 'pending' : 'none',
      );
      th.messages.push(m);
      this.ctx.emit('ai:message', { ...m });
      if (draws) this.finishDrawing(threadId, m, text);
    });
  }

  async aiChat(noteId: string, text: string, _near?: string) {
    const list = this.ctx.chat.get(noteId) ?? [];
    list.push({ id: uid(), noteId, threadId: null, author: 'you', text, drawing: 'none', svg: null, sources: [], symbols: [], trimmed: false, createdAt: now() });
    this.ctx.chat.set(noteId, list);
    this.think(noteId, text, 'chat');
    this.schedule(`${noteId}:chat`, this.wait() + 500, () => {
      const draws = DRAW.test(text);
      const m = this.msg(noteId, null, RICH.test(text) ? RICH_ANSWER : CHAT_ANSWER, draws ? 'pending' : 'none');
      list.push(m);
      this.ctx.emit('ai:message', { ...m });
      if (draws) this.finishDrawing(`${noteId}:chat`, m, text);
      this.schedule(`${noteId}:chat`, 400, () => this.foldChat(noteId));
    });
  }

  async aiCancelRead(noteId: string) {
    this.cancel(noteId);
  }
}
