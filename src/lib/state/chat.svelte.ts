import { getBackend } from '../api/backend';
import type { Message } from '../api/types';
import { caretParagraph } from '../editor/current';

export const chat = $state({
  noteId: null as string | null,
  messages: [] as Message[],
  waiting: false,
  pinned: false,
  clearedAt: 0,
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
  chat.clearedAt = 0;
  chat.fresh = {};
  chat.messages = await be.listChat(noteId);
  if (chat.noteId !== noteId) return;
  off = [
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

export const visibleChat = () => chat.messages.filter((m) => m.createdAt > chat.clearedAt);

export async function send(text: string) {
  const t = text.trim();
  if (!t || !chat.noteId) return;
  chat.messages.push({
    id: `local-${++seq}`, noteId: chat.noteId, threadId: null, author: 'you', text: t, drawing: 'none', svg: null, sources: [], createdAt: Date.now(),
  });
  chat.waiting = true;
  await (await getBackend()).aiChat(chat.noteId, t, caretParagraph());
}

export const clearChat = () => (chat.clearedAt = Date.now());

export const chatAsText = () =>
  visibleChat().map((m) => `${m.author === 'you' ? 'You' : 'Riddled'}: ${m.text}`).join('\n\n');
