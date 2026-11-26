import type { Backend, Unsubscribe } from './backend';
import { BUILTIN_RIDDLER } from './presets';
import { DEFAULT_PROVIDER_ID, DEFAULT_PROVIDERS, mergeProviders } from './providers';
import { CHAT_BUDGET, SUMMARIZE_AFTER, SUMMARIZE_FOLD, SUMMARY_MAX_CHARS, TASK_BUDGET } from './limits';
import {
  BUILTIN_RIDDLER_ID,
  type AiTask, type BackendEvents, type DeleteMode, type MathSymbol, type ExportFormat, type Folder, type Message,
  type Note, type NoteMeta, type NoteMetaPatch, type ProviderConfig, type ProviderCredentialStatus, type Riddler,
  type Settings, type Thread, type ThreadKind, type ThreadStatus, type TrashList, type ModelKind, type VoiceOption, type VoiceStatus,
} from './types';

const QUOTE = 'Photosynthesis happens in the mitochondria.';

const DEFAULT_SETTINGS: Settings = {
  newNoteMode: 'thinks',
  readingPace: 'regular',
  questionPlacement: 'both',
  webInNewNotes: false,
  voiceInNewNotes: false,
  voiceName: 'kokoro/bm_george',
  speakingSpeed: 1,
  listenModel: 'parakeet',
  appearance: 'system',
  textSize: 'default',
  pageWidth: null,
  lettering: 'own',
  inkColour: 'ink',
  justMyWords: false,
  aboutYou: '',
  learnAboutYou: true,
  searchMyNotes: false,
  explainSymbols: false,
  readProvider: DEFAULT_PROVIDER_ID,
  chatProvider: DEFAULT_PROVIDER_ID,
  drawProvider: DEFAULT_PROVIDER_ID,
  readModel: 'sonnet',
  chatModel: 'sonnet',
  drawModel: 'opus',
  readThinking: 'default',
  chatThinking: 'default',
  drawThinking: 'default',
  providers: DEFAULT_PROVIDERS.map((p) => ({ ...p, args: [...p.args], models: [...p.models] })),
  sidebarOpen: false,
  deletedNotes: 'duragon',
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
<path d="M20 30 Q40 52 62 74" stroke="var(--ink)" stroke-width="1.3" marker-end="url(#arrow)"/>
<path d="M30 20 Q52 44 74 66" stroke="var(--ink)" stroke-width="1.6"/>
<text x="10" y="14" font-size="12" fill="var(--graphite)" style="font-family:var(--font-sans)">sunlight</text>
</g>
<g data-step="2" data-caption="The chloroplast catches it">
<path d="M110 100 Q112 70 160 68 Q208 70 210 100 Q208 130 160 132 Q112 130 110 100 Z" stroke="var(--ink)" stroke-width="1.6" style="fill:var(--riddle-wash)"/>
<path d="M135 100 Q160 88 186 100" stroke="var(--faint)" stroke-width="1.2"/>
<text x="124" y="156" font-size="14" fill="var(--ink)" style="font-family:var(--font-voice)">chloroplast</text>
</g>
<g data-step="3" data-caption="Sugar and oxygen come out">
<path d="M214 98 Q250 94 288 98" stroke="var(--ink)" stroke-width="1.3" marker-end="url(#arrow)"/>
<text x="232" y="86" font-size="12" fill="var(--graphite)" style="font-family:var(--font-sans)">sugar + O2</text>
</g>
</svg>`;
}

const hash = (s: string) => [...s].reduce((h, c) => (Math.imul(h, 31) + c.charCodeAt(0)) >>> 0, 7);
const DRAW = /draw|sketch|diagram|picture|visuali[sz]e|show me/i;
const REQUEST = /^(?:draw|sketch|explain|show|tell|describe|compare|list|summari[sz]e|visuali[sz]e|what|why|how|who|when|where|can|could|should|is|are|do|does)\b/i;
const QUESTIONS = [
  'What makes you sure about that? Is there a case where it would not hold?',
  'Who is this for, and what would they need to already know?',
  'You say this as if it were settled. What would count as evidence against it?',
  'What happens at the edges here, when the input is empty or enormous?',
];
const CHAT_ANSWER = `The part I would look at again is the claim that the whole process happens in one place. Reading the note as a whole, it treats two separate jobs as one.\n\nThe first job is capturing something from outside. The second is turning it into something the rest of the system can use. They happen in different places, and the note only names one of them.\n\nA good next step is to write one sentence for each job, then see whether the note still reads the same.`;
/** Asked about a formula or code, the mock answers with every kind of rich content a reply can carry. */
const RICH = /formula|code/i;
const RICH_ANSWER = [
  'The area grows with the square of the radius, $A = \\pi r^2$, so doubling $r$ quadruples it. Summed over thin rings it is',
  '$$A = \\int_0^r 2\\pi s\\,ds = \\pi r^2$$',
  'In code, `area(r)` is one line:',
  '```python\nimport math\n\ndef area(r: float) -> float:\n    return math.pi * r ** 2  # square of the radius\n```',
  'What happens to $A$ when $r$ is negative?',
].join('\n\n');
const RICH_SYMBOLS: MathSymbol[] = [
  { tex: 'A', meaning: 'area of the circle' },
  { tex: '\\pi', meaning: 'circumference over diameter, about 3.14' },
  { tex: 'r', meaning: 'radius of the circle' },
  { tex: 's', meaning: 'radius of one thin ring' },
];

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

const KOKORO: VoiceOption[] = [
  ['bm_george', 'George', 'Warm, British'],
  ['bf_emma', 'Emma', 'Clear, British'],
  ['am_michael', 'Michael', 'Deep, steady'],
  ['af_heart', 'Heart', 'Warm, friendly'],
].map(([id, name, description]) => ({ id: `kokoro/${id}`, name, description, engine: 'Kokoro', source: 'Duragon', language: null }));

const SPEECHD: VoiceOption[] = [
  ['pico', 'Pico', 'samantha', 'en-US'],
  ['pico', 'Pico', 'serena', 'en-GB'],
  ['pico', 'Pico', 'sabrina', 'de-DE'],
  ['espeak-ng', 'eSpeak NG', 'English (America)', 'en-us'],
  ['espeak-ng', 'eSpeak NG', 'English (Great Britain)', 'en-gb'],
  ['espeak-ng', 'eSpeak NG', 'French (France)', 'fr-fr'],
].map(([module, engine, name, language]) => ({ id: `speechd/${module}/${name}`, name: name[0].toUpperCase() + name.slice(1), description: '', engine, source: 'Speech Dispatcher', language }));

const PARAKEET: VoiceOption = { id: 'parakeet', name: 'Parakeet', description: 'English', engine: 'Parakeet', source: 'Duragon', language: null };
const WHISPER: VoiceOption[] = [
  ['base.en', 'English'],
  ['small', 'Many languages'],
].map(([name, description]) => ({ id: `whisper/home/you/.local/share/whisper.cpp/ggml-${name}.bin`, name, description, engine: 'whisper.cpp', source: 'on this computer', language: null }));


/** What one trash file holds. */
interface Trashed {
  note: Note;
  threads: Thread[];
  chat: Message[];
  summary: { text: string; through: string | null } | undefined;
  deletedAt: number;
}

export class MockBackend implements Backend {
  private notes = new Map<string, Note>();
  private folders = new Map<string, Folder>();
  private threads = new Map<string, Thread>();
  private chat = new Map<string, Message[]>();
  private summaries = new Map<string, { text: string; through: string | null }>();
  private trash = new Map<string, Trashed>();
  private systemTrash = new Map<string, Trashed>();
  private riddlers = new Map<string, Riddler>([[BUILTIN_RIDDLER_ID, BUILTIN_RIDDLER]]);
  private settings: Settings = structuredClone(DEFAULT_SETTINGS);
  private keys = new Map<string, string>();
  private handlers = new Map<string, Set<(p: any) => void>>();
  private timers = new Map<string, number[]>();

  /** Test hook: the system trash takes nothing. */
  failSystemTrash = false;
  /** Test hook: how many files in the trash can't be read. */
  unreadableTrash = 0;
  /** Test hook: the user takes the note back out of the system trash. */
  restoreFromSystem(id: string) {
    const file = this.systemTrash.get(id);
    if (!file) throw new Error(`not in the system trash: ${id}`);
    this.systemTrash.delete(id);
    this.trash.set(id, file);
  }

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
          symbols: [],
          trimmed: false,
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
  /** Like Trash::delete. */
  async deleteNote(id: string, mode: DeleteMode): Promise<DeleteMode> {
    const note = this.notes.get(id);
    if (note) {
      const threads = [...this.threads.values()].filter((t) => t.noteId === id);
      if (mode === 'permanent') this.trash.delete(id);
      else this.trash.set(id, { note, threads, chat: this.chat.get(id) ?? [], summary: this.summaries.get(id), deletedAt: now() });
      this.notes.delete(id);
      for (const t of threads) this.threads.delete(t.id);
      this.chat.delete(id);
      this.summaries.delete(id);
    }
    const file = this.trash.get(id);
    if (mode !== 'system' || !file) return mode;
    if (this.failSystemTrash) return 'duragon';
    this.trash.delete(id);
    this.systemTrash.set(id, file);
    return 'system';
  }

  async listTrash(): Promise<TrashList> {
    const notes = [...this.trash.values()]
      .filter((f) => !this.notes.has(f.note.id))
      .sort((a, b) => b.deletedAt - a.deletedAt)
      .map((f) => ({ id: f.note.id, title: f.note.title, deletedAt: f.deletedAt }));
    return { notes, unreadable: this.unreadableTrash };
  }
  /** Like Trash::restore. */
  async restoreNote(id: string) {
    const live = this.notes.get(id);
    if (live) return this.meta(live);
    const file = this.trash.get(id);
    if (!file) throw new Error('that note is not in the trash');
    const { note, threads, chat, summary } = file;
    if (note.folderId && !this.folders.has(note.folderId)) note.folderId = null;
    if (!this.riddlers.has(note.riddlerId)) note.riddlerId = BUILTIN_RIDDLER_ID;
    for (const m of [...threads.flatMap((t) => t.messages), ...chat]) if (m.drawing === 'pending') m.drawing = 'failed';
    this.notes.set(id, note);
    for (const t of threads) this.threads.set(t.id, t);
    this.chat.set(id, chat);
    if (summary) this.summaries.set(id, summary);
    this.trash.delete(id);
    return this.meta(note);
  }
  async deleteTrashed(ids: string[]) {
    for (const id of ids) if (!this.notes.has(id)) this.trash.delete(id);
  }

  async listFolders() {
    return [...this.folders.values()];
  }
  async createFolder(name: string) {
    const f: Folder = { id: uid(), name, parentId: null, createdAt: now() };
    this.folders.set(f.id, f);
    return f;
  }
  async renameFolder(id: string, name: string) {
    const f = this.folders.get(id);
    if (f) f.name = name;
  }
  async moveFolder(id: string, parentId: string | null) {
    for (let at = parentId; at !== null; at = this.folders.get(at)?.parentId ?? null) {
      if (at === id) throw new Error("a folder can't go inside itself");
    }
    const f = this.folders.get(id);
    if (f) f.parentId = parentId;
  }
  async deleteFolder(id: string) {
    const up = this.folders.get(id)?.parentId ?? null;
    this.folders.delete(id);
    for (const n of this.notes.values()) if (n.folderId === id) n.folderId = up;
    for (const f of this.folders.values()) if (f.parentId === id) f.parentId = up;
  }

  async listThreads(noteId: string) {
    return structuredClone([...this.threads.values()].filter((t) => t.noteId === noteId));
  }
  async setThreadStatus(id: string, status: ThreadStatus) {
    const t = this.threads.get(id);
    if (t) t.status = status;
  }
  async setThreadKind(id: string, kind: ThreadKind) {
    const t = this.threads.get(id);
    if (t) t.kind = kind;
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
  async clearChat(noteId: string) {
    this.cancel(`${noteId}:chat`);
    this.chat.delete(noteId);
    this.summaries.delete(noteId);
  }
  async getChatSummary(noteId: string) {
    return this.summaries.get(noteId)?.text ?? '';
  }
  async saveChatSummary(noteId: string, base: string, text: string) {
    const prev = this.summaries.get(noteId) ?? { text: '', through: null };
    if (prev.text !== base) return false;
    this.summaries.set(noteId, { text: text.slice(0, SUMMARY_MAX_CHARS), through: prev.through });
    return true;
  }

  /** Like Ai::summarize. */
  private foldChat(noteId: string) {
    const list = this.chat.get(noteId) ?? [];
    const prev = this.summaries.get(noteId) ?? { text: '', through: null };
    const pending = list.slice(list.findIndex((m) => m.id === prev.through) + 1);
    if (pending.length <= SUMMARIZE_AFTER) return;
    const fold = pending.slice(0, SUMMARIZE_FOLD);
    const asked = fold.filter((m) => m.author === 'you').map((m) => `“${m.text}”`).join(', ');
    const text = `${prev.text ? `${prev.text} ` : ''}The writer asked ${asked}, and the replies pointed at the claim that the whole process happens in one place.`;
    this.summaries.set(noteId, { text, through: fold[fold.length - 1].id });
    this.emit('chat:summary', { noteId, summary: text });
  }

  async getSettings() {
    return structuredClone(this.settings);
  }
  async saveSettings(settings: Settings) {
    const kept = new Set(mergeProviders(settings.providers).map((p) => p.id));
    for (const { id } of this.providers()) if (!kept.has(id)) this.keys.delete(id);
    this.settings = structuredClone(settings);
  }
  private providers() {
    return mergeProviders(this.settings.providers);
  }
  private knownProvider(id: string) {
    const p = this.providers().find((x) => x.id === id);
    if (!p) throw new Error(`there is no provider "${id}"`);
    return p;
  }
  private credentialStatus(p: ProviderConfig): ProviderCredentialStatus {
    const stored = this.keys.get(p.id);
    const maskedKey = stored
      ? stored.length <= 8 ? '••••' : `${stored.slice(0, 4)}…${stored.slice(-4)}`
      : null;
    return {
      providerId: p.id,
      hasStoredKey: !!stored,
      maskedKey,
      envVar: p.envVar ?? null,
      hasEnvKey: false,
    };
  }
  async listCredentialStatuses() {
    return this.providers().map((p) => this.credentialStatus(p));
  }
  async localModels(providerId: string) {
    const p = this.settings.providers.find((x) => x.id === providerId);
    if (p?.kind === 'ollama' && p.baseUrl === 'http://localhost:11434') return ['qwen3:8b', 'llama3.2:3b'];
    throw `Nothing answers at ${p?.baseUrl}`;
  }
  async setProviderKey(providerId: string, key: string) {
    const p = this.knownProvider(providerId);
    const t = key.trim();
    if (t) this.keys.set(providerId, t);
    else this.keys.delete(providerId);
    return this.credentialStatus(p);
  }
  async deleteProviderKey(providerId: string) {
    const p = this.knownProvider(providerId);
    this.keys.delete(providerId);
    return this.credentialStatus(p);
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
    const symbols = this.settings.explainSymbols && text === RICH_ANSWER ? RICH_SYMBOLS : [];
    const note = this.notes.get(noteId);
    const budget = threadId === null ? CHAT_BUDGET : TASK_BUDGET;
    const trimmed = !!note && Math.ceil(plainOf(note.doc).length / 4) > budget;
    return { id: uid(), noteId, threadId, author: 'riddled', text, drawing, svg: null, sources, symbols, trimmed, createdAt: now() };
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
    if (!this.notes.has(noteId)) return;
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

  private answerText(topic: string) {
    if (RICH.test(topic)) return RICH_ANSWER;
    return `Here is how I would put it: ${topic.replace(/[.?!]+$/, '')} comes down to a few moving parts that depend on each other. Tell me where that does not fit what you had in mind.`;
  }

  /** Like the real read pass: questions and requests get an inline reply, other prose a margin question or nothing. */
  async aiRead(noteId: string, target: string) {
    this.think(noteId, target, 'read');
    this.schedule(noteId, 1500 + Math.random() * 1500, () => {
      if (target.endsWith('?') || REQUEST.test(target)) return this.openThread(noteId, 'reply', target, this.answerText(target), DRAW.test(target), target);
      if (hash(target) % 4 === 0) return this.emit('ai:quiet', { noteId, target });
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
    const th = this.threads.get(threadId);
    if (!th) throw new Error(`thread not found: ${threadId}`);
    th.messages.push({ id: uid(), noteId: th.noteId, threadId, author: 'you', text, drawing: 'none', svg: null, sources: [], symbols: [], trimmed: false, createdAt: now() });
    this.think(th.noteId, text, 'reply');
    this.schedule(threadId, this.wait(), () => {
      const draws = DRAW.test(text);
      const m = this.msg(th.noteId, threadId, draws ? 'Let me show that as a picture. Watch the order of the steps, because the order is the point.' : RICH.test(text) ? RICH_ANSWER : 'That makes sense. What would change your mind?', draws ? 'pending' : 'none');
      th.messages.push(m);
      this.emit('ai:message', { ...m });
      if (draws) this.finishDrawing(threadId, m, text);
    });
  }
  async aiChat(noteId: string, text: string, _near?: string) {
    const list = this.chat.get(noteId) ?? [];
    list.push({ id: uid(), noteId, threadId: null, author: 'you', text, drawing: 'none', svg: null, sources: [], symbols: [], trimmed: false, createdAt: now() });
    this.chat.set(noteId, list);
    this.think(noteId, text, 'chat');
    this.schedule(`${noteId}:chat`, this.wait() + 500, () => {
      const draws = DRAW.test(text);
      const m = this.msg(noteId, null, RICH.test(text) ? RICH_ANSWER : CHAT_ANSWER, draws ? 'pending' : 'none');
      list.push(m);
      this.emit('ai:message', { ...m });
      if (draws) this.finishDrawing(`${noteId}:chat`, m, text);
      this.schedule(`${noteId}:chat`, 400, () => this.foldChat(noteId));
    });
  }
  async aiCancelRead(noteId: string) {
    this.cancel(noteId);
  }
  private cancel(key: string) {
    this.timers.get(key)?.forEach(clearTimeout);
    this.timers.delete(key);
  }

  private voice: VoiceStatus = { stt: 'missing', vad: 'missing', tts: 'missing', progress: 0, error: null, listening: false };
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
  async voiceInstall(kind: ModelKind) {
    const v = this.voice;
    if (v.stt === 'downloading' || v.vad === 'downloading' || v.tts === 'downloading') return;
    this.setVoice({ [kind]: 'downloading', progress: 0, error: null });
    const start = performance.now();
    const ms = { stt: 3000, vad: 600, tts: 2000 }[kind];
    const tick = window.setInterval(() => {
      const p = Math.min(1, (performance.now() - start) / ms);
      if (p >= 1) {
        clearInterval(tick);
        this.setVoice({ [kind]: 'ready', progress: 1 });
      } else this.setVoice({ progress: p });
    }, 100);
  }
  async listVoices(_refresh: boolean) {
    return [...(this.voice.tts === 'ready' ? KOKORO : []), ...SPEECHD].map((v) => ({ ...v }));
  }
  async listListenModels() {
    return [...(this.voice.stt === 'ready' ? [PARAKEET] : []), ...WHISPER].map((m) => ({ ...m }));
  }
  async listenStart(handsFree: boolean) {
    if (handsFree && this.voice.vad !== 'ready') throw new Error('Hands-free listening needs Silero VAD. Install it in Customize → Voice models.');
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
  async speak(messageId: string | null, _text: string, _voice?: string) {
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

