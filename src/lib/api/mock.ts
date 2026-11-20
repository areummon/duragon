import type { Backend, Unsubscribe } from './backend';
import { BUILTIN_RIDDLER } from './presets';
import {
  BUILTIN_RIDDLER_ID,
  type AiTask, type BackendEvents, type ExportFormat, type Folder, type Message,
  type Note, type NoteMeta, type NoteMetaPatch, type Riddler, type Settings,
  type Thread, type ThreadStatus, type VoiceOption, type VoiceStatus,
} from './types';

const QUOTE = 'Photosynthesis happens in the mitochondria.';

const DEFAULT_SETTINGS: Settings = {
  newNoteMode: 'thinks',
  readingPace: 'regular',
  questionPlacement: 'both',
  webInNewNotes: false,
  voiceInNewNotes: false,
  voiceName: 'bm_george',
  speakingSpeed: 1,
  appearance: 'system',
  textSize: 'default',
  pageWidth: null,
  lettering: 'own',
  inkColour: 'ink',
  inkFromPointer: false,
  justMyWords: false,
  aboutYou: '',
  learnAboutYou: true,
  readModel: 'sonnet',
  chatModel: 'sonnet',
  drawModel: 'sonnet',
  sidebarOpen: false,
};

const uid = () => crypto.randomUUID();
const para = (text: string) => ({ type: 'paragraph', content: [{ type: 'text', text }] });
const title = (text: string) => ({ type: 'title', content: [{ type: 'text', text }] });

function plainOf(doc: unknown): string {
  const out: string[] = [];
  const walk = (n: any) => {
    if (n.type === 'text') out.push(n.text);
    n.content?.forEach(walk);
    if (n.type !== 'doc' && n.type !== 'text') out.push('\n');
  };
  walk(doc);
  return out.join('').trim();
}

export function sampleDrawing(): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 170" style="fill:none;stroke-linecap:round;stroke-linejoin:round">
<g data-step="1" data-caption="Light comes in">
<path d="M20 30 Q40 52 62 74" stroke="var(--ink)" stroke-width="1.6"/>
<path d="M30 20 Q52 44 74 66" stroke="var(--ink)" stroke-width="1.6"/>
<path d="M62 74 L52 72 M62 74 L63 63" stroke="var(--ink)" stroke-width="1.6"/>
<text x="10" y="14" font-size="12" fill="var(--graphite)" style="font-family:var(--font-sans)">sunlight</text>
</g>
<g data-step="2" data-caption="The chloroplast catches it">
<path d="M110 100 Q112 70 160 68 Q208 70 210 100 Q208 130 160 132 Q112 130 110 100 Z" stroke="var(--ink)" stroke-width="1.6" style="fill:var(--riddle-wash)"/>
<path d="M135 100 Q160 88 186 100" stroke="var(--faint)" stroke-width="1.2"/>
<text x="124" y="156" font-size="14" fill="var(--ink)" style="font-family:var(--font-voice)">chloroplast</text>
</g>
<g data-step="3" data-caption="Sugar and oxygen come out">
<path d="M214 98 Q250 94 288 98" stroke="var(--ink)" stroke-width="1.6"/>
<path d="M288 98 L278 92 M288 98 L278 105" stroke="var(--ink)" stroke-width="1.6"/>
<text x="232" y="86" font-size="12" fill="var(--graphite)" style="font-family:var(--font-sans)">sugar + O2</text>
</g>
</svg>`;
}

const hash = (s: string) => [...s].reduce((h, c) => (Math.imul(h, 31) + c.charCodeAt(0)) >>> 0, 7);
const DRAW = /draw|sketch|diagram|picture|visuali[sz]e|show me/i;
const QUESTIONS = [
  'What makes you sure about that? Is there a case where it would not hold?',
  'Who is this for, and what would they need to already know?',
  'You say this as if it were settled. What would count as evidence against it?',
  'What happens at the edges here, when the input is empty or enormous?',
];
const CHAT_ANSWER = `The part I would look at again is the claim that the whole process happens in one place. Reading the note as a whole, it treats two separate jobs as one.\n\nThe first job is capturing something from outside. The second is turning it into something the rest of the system can use. They happen in different places, and the note only names one of them.\n\nA good next step is to write one sentence for each job, then see whether the note still reads the same.`;

export function cellDrawing(): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 210" style="fill:none;stroke-linecap:round;stroke-linejoin:round">
<g data-step="1" data-caption="A membrane holds the cell together">
<path d="M60 105 Q58 50 160 46 Q262 50 260 105 Q262 162 160 166 Q58 162 60 105 Z" stroke="var(--ink)" stroke-width="1.8" style="fill:var(--riddle-wash)"/>
<text x="118" y="30" font-size="14" fill="var(--ink)" style="font-family:var(--font-voice)">cell membrane</text>
</g>
<g data-step="2" data-caption="The nucleus keeps the instructions">
<path d="M130 90 Q132 68 158 68 Q184 70 184 92 Q182 114 156 114 Q130 112 130 90 Z" stroke="var(--ink)" stroke-width="1.6"/>
<path d="M146 90 Q158 82 170 92" stroke="var(--faint)" stroke-width="1.2"/>
<text x="124" y="136" font-size="13" fill="var(--graphite)" style="font-family:var(--font-sans)">nucleus</text>
</g>
<g data-step="3" data-caption="Mitochondria turn sugar into usable energy">
<path d="M206 118 Q206 104 222 104 Q240 104 240 118 Q240 132 222 132 Q206 132 206 118 Z" stroke="var(--ink)" stroke-width="1.5"/>
<path d="M212 118 Q218 110 224 120 Q230 126 236 116" stroke="var(--faint)" stroke-width="1.2"/>
<text x="196" y="152" font-size="13" fill="var(--graphite)" style="font-family:var(--font-sans)">mitochondrion</text>
</g>
<g data-step="4" data-caption="Together they keep the cell running">
<path d="M262 80 Q286 78 300 96" stroke="var(--ink)" stroke-width="1.5"/>
<path d="M300 96 L289 94 M300 96 L297 85" stroke="var(--ink)" stroke-width="1.5"/>
<text x="268" y="70" font-size="12" fill="var(--graphite)" style="font-family:var(--font-sans)">energy</text>
</g>
</svg>`;
}

function drawingFor(topic: string): string {
  return /cell/i.test(topic) ? cellDrawing() : sampleDrawing();
}

const now = () => Date.now();

const SOURCES = [
  { url: 'https://en.wikipedia.org/wiki/Photosynthesis', title: 'Photosynthesis, Wikipedia' },
  { url: 'https://www.khanacademy.org/science/biology', title: 'Biology, Khan Academy' },
];

const VOICES: VoiceOption[] = [
  { id: 'bm_george', name: 'George', description: 'Warm, British' },
  { id: 'bf_emma', name: 'Emma', description: 'Clear, British' },
  { id: 'am_michael', name: 'Michael', description: 'Deep, steady' },
  { id: 'af_heart', name: 'Heart', description: 'Warm, friendly' },
];

export class MockBackend implements Backend {
  private notes = new Map<string, Note>();
  private folders = new Map<string, Folder>();
  private threads = new Map<string, Thread>();
  private chat = new Map<string, Message[]>();
  private riddlers = new Map<string, Riddler>([[BUILTIN_RIDDLER_ID, BUILTIN_RIDDLER]]);
  private settings: Settings = { ...DEFAULT_SETTINGS };
  private handlers = new Map<string, Set<(p: any) => void>>();
  private timers = new Map<string, number[]>();

  constructor() {
    this.seed();
  }

  private seed() {
    const t = now();
    const welcome: Note = {
      id: uid(),
      title: 'Welcome to Duragon',
      folderId: null,
      updatedAt: t,
      hasOpenQuestion: true,
      doc: {
        type: 'doc',
        content: [
          title('Welcome to Duragon'),
          para('Duragon is a quiet notebook. You write, and Claude reads along. When something is worth a second look, it asks a question in the margin instead of interrupting.'),
          para('Address it by name and it answers right under your line. When a picture would help, it sketches one. You can also open the chat to talk about the whole note.'),
          para(`Your notes live on this machine. Try writing something wrong on purpose. ${QUOTE} Then see what comes back.`),
        ],
      },
      riddlerId: BUILTIN_RIDDLER_ID,
      mode: 'thinks',
      voiceOn: false,
      webOn: false,
      brief: '',
      createdAt: t,
    };
    const scratch: Note = {
      id: uid(),
      title: 'Scratch',
      folderId: null,
      updatedAt: t - 3_600_000,
      hasOpenQuestion: false,
      doc: { type: 'doc', content: [title('Scratch'), para('A second note, to fill the list.')] },
      riddlerId: BUILTIN_RIDDLER_ID,
      mode: 'thinks',
      voiceOn: false,
      webOn: false,
      brief: '',
      createdAt: t - 3_600_000,
    };
    this.notes.set(welcome.id, welcome);
    this.notes.set(scratch.id, scratch);
    const threadId = uid();
    this.threads.set(threadId, {
      id: threadId,
      noteId: welcome.id,
      kind: 'question',
      quote: QUOTE,
      status: 'open',
      pinned: false,
      createdAt: t,
      messages: [
        {
          id: uid(),
          noteId: welcome.id,
          threadId,
          author: 'riddled',
          text: 'Is that where it happens? What does the mitochondria do, and which part of the cell handles light?',
          drawing: 'none',
          svg: null,
          sources: [],
          createdAt: t,
        },
      ],
    });
  }

  private emit<K extends keyof BackendEvents>(event: K, payload: BackendEvents[K]) {
    this.handlers.get(event)?.forEach((h) => h(payload));
  }

  private schedule(key: string, ms: number, fn: () => void) {
    const id = window.setTimeout(fn, ms);
    this.timers.set(key, [...(this.timers.get(key) ?? []), id]);
  }

  private meta(n: Note): NoteMeta {
    const open = [...this.threads.values()].some((t) => t.noteId === n.id && t.status === 'open');
    return { id: n.id, title: n.title, folderId: n.folderId, updatedAt: n.updatedAt, hasOpenQuestion: open };
  }

  private requireNote(id: string): Note {
    const n = this.notes.get(id);
    if (!n) throw new Error(`note not found: ${id}`);
    return n;
  }

  async listNotes() {
    return [...this.notes.values()].sort((a, b) => b.updatedAt - a.updatedAt).map((n) => this.meta(n));
  }
  async searchNotes(query: string) {
    const q = query.trim().toLowerCase();
    return (await this.listNotes()).filter((m) => {
      const n = this.notes.get(m.id)!;
      return q === '' || m.title.toLowerCase().includes(q) || plainOf(n.doc).toLowerCase().includes(q);
    });
  }
  async getNote(id: string) {
    return structuredClone({ ...this.requireNote(id), ...this.meta(this.requireNote(id)) });
  }
  async createNote(titleText = '') {
    const t = now();
    const s = this.settings;
    const n: Note = {
      id: uid(), title: titleText, folderId: null, updatedAt: t, hasOpenQuestion: false,
      doc: { type: 'doc', content: [title(titleText), { type: 'paragraph' }] },
      riddlerId: BUILTIN_RIDDLER_ID, mode: s.newNoteMode, voiceOn: s.voiceInNewNotes,
      webOn: s.webInNewNotes, brief: '', createdAt: t,
    };
    this.notes.set(n.id, n);
    return structuredClone(n);
  }
  async saveNote(id: string, titleText: string, doc: unknown, _plain: string) {
    const n = this.requireNote(id);
    Object.assign(n, { title: titleText, doc, updatedAt: now() });
    return this.meta(n);
  }
  async updateNoteMeta(id: string, patch: NoteMetaPatch) {
    const n = this.requireNote(id);
    Object.assign(n, Object.fromEntries(Object.entries(patch).filter(([, v]) => v !== undefined)));
    return this.getNote(id);
  }
  async deleteNote(id: string) {
    this.notes.delete(id);
    for (const [tid, t] of this.threads) if (t.noteId === id) this.threads.delete(tid);
    this.chat.delete(id);
  }

  async listFolders() {
    return [...this.folders.values()];
  }
  async createFolder(name: string) {
    const f = { id: uid(), name, createdAt: now() };
    this.folders.set(f.id, f);
    return f;
  }
  async renameFolder(id: string, name: string) {
    const f = this.folders.get(id);
    if (f) f.name = name;
  }
  async deleteFolder(id: string) {
    this.folders.delete(id);
    for (const n of this.notes.values()) if (n.folderId === id) n.folderId = null;
  }

  async listThreads(noteId: string) {
    return structuredClone([...this.threads.values()].filter((t) => t.noteId === noteId));
  }
  async setThreadStatus(id: string, status: ThreadStatus) {
    const t = this.threads.get(id);
    if (t) t.status = status;
  }
  async setThreadPinned(id: string, pinned: boolean) {
    const t = this.threads.get(id);
    if (t) t.pinned = pinned;
  }
  async threadToNote(id: string) {
    const t = this.threads.get(id);
    if (!t) throw new Error(`thread not found: ${id}`);
    const n = await this.createNote(t.quote.slice(0, 60));
    n.doc = {
      type: 'doc',
      content: [title(n.title), ...t.messages.map((m) => para(m.text))],
    };
    this.notes.set(n.id, n);
    return structuredClone(n);
  }
  async listChat(noteId: string) {
    return structuredClone(this.chat.get(noteId) ?? []);
  }

  async getSettings() {
    return { ...this.settings };
  }
  async saveSettings(settings: Settings) {
    this.settings = { ...settings };
  }
  async listRiddlers() {
    return [...this.riddlers.values()];
  }
  async saveRiddler(riddler: Riddler) {
    const saved = { ...riddler, id: riddler.id || uid(), builtin: false };
    this.riddlers.set(saved.id, saved);
    return saved;
  }
  async deleteRiddler(id: string) {
    if (id !== BUILTIN_RIDDLER_ID) this.riddlers.delete(id);
  }

  private msg(noteId: string, threadId: string | null, text: string, drawing: Message['drawing'] = 'none'): Message {
    const web = this.notes.get(noteId)?.webOn ?? false;
    const sources = web ? SOURCES.slice(0, 1 + (hash(text) % 2)) : [];
    return { id: uid(), noteId, threadId, author: 'riddled', text, drawing, svg: null, sources, createdAt: now() };
  }

  private think(noteId: string, target: string, task: AiTask) {
    this.emit('ai:thinking', { noteId, target, task });
  }

  private wait() {
    return 1200 + Math.random() * 1300;
  }

  /** Drawing arrives later: the message is emitted as pending, then as done with the svg. */
  private finishDrawing(key: string, m: Message, topic: string) {
    this.schedule(key, 1800 + Math.random() * 700, () => {
      m.drawing = 'done';
      m.svg = drawingFor(topic);
      this.emit('ai:message', { ...m });
    });
  }

  private openThread(noteId: string, kind: Thread['kind'], quote: string, text: string, wantsDrawing: boolean, topic: string) {
    const th: Thread = { id: uid(), noteId, kind, quote, status: 'open', pinned: false, createdAt: now(), messages: [] };
    const m = this.msg(noteId, th.id, text, wantsDrawing ? 'pending' : 'none');
    th.messages.push(m);
    this.threads.set(th.id, th);
    this.emit('ai:thread', structuredClone(th));
    if (wantsDrawing) this.finishDrawing(th.id, m, topic);
  }

  private firstSentence(target: string) {
    const s = target.match(/[^.!?]+[.!?]?/)?.[0].trim() ?? target;
    return (s.length >= 12 ? s : target).slice(0, 90).trim();
  }

  async aiRead(noteId: string, target: string) {
    this.think(noteId, target, 'read');
    this.schedule(noteId, this.wait(), () => {
      if (hash(target) % 4 === 0) return this.emit('ai:quiet', { noteId, target });
      const q = QUESTIONS[hash(target) % QUESTIONS.length];
      this.openThread(noteId, 'question', this.firstSentence(target), q, DRAW.test(target), target);
    });
  }
  async aiAnswer(noteId: string, target: string) {
    this.think(noteId, target, 'answer');
    this.schedule(noteId, this.wait(), () => {
      const topic = target.replace(/^\s*(hey\s+)?\w+\s*[,:]\s*/i, '');
      this.openThread(noteId, 'reply', target, `Here is how I would put it: ${topic.replace(/[.?!]+$/, '')} comes down to a few moving parts that depend on each other. Tell me where that does not fit what you had in mind.`, DRAW.test(target), topic);
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
    const th = this.threads.get(threadId);
    if (!th) throw new Error(`thread not found: ${threadId}`);
    th.messages.push({ id: uid(), noteId: th.noteId, threadId, author: 'you', text, drawing: 'none', svg: null, sources: [], createdAt: now() });
    this.think(th.noteId, text, 'reply');
    this.schedule(threadId, this.wait(), () => {
      const draws = DRAW.test(text);
      const m = this.msg(th.noteId, threadId, draws ? 'Let me show that as a picture. Watch the order of the steps, because the order is the point.' : 'That makes sense. What would change your mind?', draws ? 'pending' : 'none');
      th.messages.push(m);
      this.emit('ai:message', { ...m });
      if (draws) this.finishDrawing(threadId, m, text);
    });
  }
  async aiChat(noteId: string, text: string, _near?: string) {
    const list = this.chat.get(noteId) ?? [];
    list.push({ id: uid(), noteId, threadId: null, author: 'you', text, drawing: 'none', svg: null, sources: [], createdAt: now() });
    this.chat.set(noteId, list);
    this.think(noteId, text, 'chat');
    this.schedule(`${noteId}:chat`, this.wait() + 500, () => {
      const draws = DRAW.test(text);
      const m = this.msg(noteId, null, CHAT_ANSWER, draws ? 'pending' : 'none');
      list.push(m);
      this.emit('ai:message', { ...m });
      if (draws) this.finishDrawing(`${noteId}:chat`, m, text);
    });
  }
  async aiCancelRead(noteId: string) {
    this.timers.get(noteId)?.forEach(clearTimeout);
    this.timers.delete(noteId);
  }

  private voice: VoiceStatus = { stt: 'missing', tts: 'missing', progress: 0, error: null, listening: false };
  private levelTimer = 0;
  private transcriptTimer = 0;
  private speakTimer = 0;

  private setVoice(patch: Partial<VoiceStatus>) {
    this.voice = { ...this.voice, ...patch };
    this.emit('voice:status', { ...this.voice });
  }
  async voiceStatus() {
    return { ...this.voice };
  }
  async voiceInstall() {
    if (this.voice.stt === 'downloading') return;
    this.setVoice({ stt: 'downloading', tts: 'downloading', progress: 0, error: null });
    const start = performance.now();
    const tick = window.setInterval(() => {
      const p = Math.min(1, (performance.now() - start) / 3000);
      if (p >= 1) {
        clearInterval(tick);
        this.setVoice({ stt: 'ready', tts: 'ready', progress: 1 });
      } else this.setVoice({ progress: p });
    }, 100);
  }
  async listVoices() {
    return VOICES.map((v) => ({ ...v }));
  }
  async listenStart(handsFree: boolean) {
    this.setVoice({ listening: true });
    this.levelTimer = window.setInterval(() => this.emit('voice:level', Math.random() * 0.8 + 0.1), 50);
    this.transcriptTimer = window.setTimeout(() => {
      this.emit('voice:transcript', { text: 'This is what I said out loud.', final: true });
      if (handsFree) this.transcriptTimer = window.setTimeout(() => this.emit('voice:transcript', { text: 'And here is a second thought.', final: true }), 2000);
    }, 2000);
  }
  async listenStop() {
    clearInterval(this.levelTimer);
    clearTimeout(this.transcriptTimer);
    this.emit('voice:level', 0);
    this.setVoice({ listening: false });
    return 'This is what I said out loud.';
  }
  async speak(messageId: string | null, _text: string) {
    clearTimeout(this.speakTimer);
    this.emit('voice:speaking', { messageId, playing: true });
    this.speakTimer = window.setTimeout(() => this.emit('voice:speaking', { messageId, playing: false }), 3000);
  }
  async stopSpeaking() {
    clearTimeout(this.speakTimer);
    this.emit('voice:speaking', { messageId: null, playing: false });
  }

  async exportNote(_id: string, _format: ExportFormat, _path: string) {}

  on<K extends keyof BackendEvents>(event: K, handler: (payload: BackendEvents[K]) => void): Unsubscribe {
    const set = this.handlers.get(event) ?? new Set();
    set.add(handler);
    this.handlers.set(event, set);
    return () => set.delete(handler);
  }
}

