import { getBackend } from '../api/backend';
import type { Message } from '../api/types';
import { caretParagraph } from '../editor/current';

export const chat = $state({
  noteId: null as string | null,
  messages: [] as Message[],
  waiting: false,
  pinned: false,
  /** What the backend folded of the older messages; '' until it has. */
  summary: '',
  fresh: {} as Record<string, true>,
});

let off: (() => void)[] = [];
let seq = 0;

export async function loadChat(noteId: string) {
  off.forEach((u) => u());
  off = [];
  const be = await getBackend();
  chat.noteId = noteId;
  chat.waiting = false;
  chat.fresh = {};
  chat.summary = '';
  const [messages, summary] = await Promise.all([be.listChat(noteId), be.getChatSummary(noteId)]);
  if (chat.noteId !== noteId) return;
  chat.messages = messages;
  chat.summary = summary;
  off = [
    be.on('chat:summary', (p) => {
      if (p.noteId === noteId) chat.summary = p.summary;
    }),
    be.on('ai:message', (m) => {
      if (m.noteId !== noteId || m.threadId !== null) return;
      const i = chat.messages.findIndex((x) => x.id === m.id || (x.id.startsWith('local-') && x.author === m.author && x.text === m.text));
      if (i >= 0) {
        if (m.author === 'riddled' && m.drawing === 'done' && chat.messages[i].drawing !== 'done') chat.fresh[`d${m.id}`] = true;
        chat.messages[i] = m;
      } else {
        chat.messages.push(m);
        if (m.author === 'riddled') chat.fresh[m.id] = true;
        if (m.drawing === 'done') chat.fresh[`d${m.id}`] = true;
      }
      if (m.author === 'riddled') chat.waiting = false;
    }),
    be.on('ai:error', (p) => {
      if (p.noteId === noteId && p.task === 'chat') chat.waiting = false;
    }),
  ];
}

export async function send(text: string) {
  const t = text.trim();
  if (!t || !chat.noteId) return;
  const id = `local-${++seq}`;
  chat.messages.push({
    id, noteId: chat.noteId, threadId: null, author: 'you', text: t, drawing: 'none', svg: null, sources: [], symbols: [], trimmed: false, createdAt: Date.now(),
  });
  chat.fresh[id] = true;
  chat.waiting = true;
  await (await getBackend()).aiChat(chat.noteId, t, caretParagraph());
}

export async function saveSummary(noteId: string, text: string) {
  if (chat.noteId === noteId) chat.summary = text;
  await (await getBackend()).saveChatSummary(noteId, text);
}

export async function clearChat() {
  const noteId = chat.noteId;
  if (!noteId) return;
  chat.messages = [];
  chat.summary = '';
  chat.fresh = {};
  await (await getBackend()).clearChat(noteId);
}

export const chatAsText = () =>
  chat.messages.map((m) => `${m.author === 'you' ? 'You' : 'Riddled'}: ${m.text}`).join('\n\n');
