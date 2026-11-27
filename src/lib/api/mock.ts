import type { Backend, Unsubscribe } from './backend';
import { DEFAULT_SETTINGS } from './defaults';
import { SUMMARY_MAX_CHARS } from './limits';
import { MockAi } from './mock/ai';
import {
  type StoredRevision,
  cellDrawing,
  createSeedData,
  hasNoteLink,
  maskKey,
  now,
  para,
  plainOf,
  sampleDrawing,
  title,
  uid,
} from './mock/fixtures';
import { MockVoice } from './mock/voice';
import { BUILTIN_RIDDLER } from './presets';
import { mergeProviders } from './providers';
import { SEARCH_ENGINES } from './search';
import {
  BUILTIN_RIDDLER_ID,
  type BackendEvents,
  type DeleteMode,
  type ExportFormat,
  type Folder,
  type Message,
  type ModelKind,
  type Note,
  type NoteMeta,
  type NoteMetaPatch,
  type NoteRevision,
  type ProviderConfig,
  type ProviderCredentialStatus,
  type Riddler,
  type SearchEngine,
  type SearchEngineCredentialStatus,
  type SearchStatus,
  type Settings,
  type Thread,
  type ThreadKind,
  type ThreadStatus,
  type TrashList,
} from './types';

export { cellDrawing, sampleDrawing };

/** What one trash file holds. */
interface Trashed {
  note: Note;
  threads: Thread[];
  chat: Message[];
  summary: { text: string; through: string | null } | undefined;
  deletedAt: number;
}

const REVISION_INTERVAL_MS = 180_000;
const MAX_REVISIONS_PER_NOTE = 20;

export class MockBackend implements Backend {
  private notes = new Map<string, Note>();
  private revisions = new Map<string, StoredRevision[]>();
  private folders = new Map<string, Folder>();
  private threads = new Map<string, Thread>();
  private chat = new Map<string, Message[]>();
  private summaries = new Map<string, { text: string; through: string | null }>();
  private trash = new Map<string, Trashed>();
  private systemTrash = new Map<string, Trashed>();
  private riddlers = new Map<string, Riddler>([[BUILTIN_RIDDLER_ID, BUILTIN_RIDDLER]]);
  private settings: Settings = structuredClone(DEFAULT_SETTINGS);
  private keys = new Map<string, string>();
  private searchKeys = new Map<SearchEngine, string>();
  private handlers = new Map<keyof BackendEvents, Set<(payload: never) => void>>();
  private ai: MockAi;
  private voice: MockVoice;

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
    const emit = <K extends keyof BackendEvents>(event: K, payload: BackendEvents[K]) => {
      const set = this.handlers.get(event) as Set<(p: BackendEvents[K]) => void> | undefined;
      set?.forEach((h) => h(payload));
    };
    this.ai = new MockAi({
      notes: this.notes,
      threads: this.threads,
      chat: this.chat,
      summaries: this.summaries,
      getSettings: () => this.settings,
      emit,
    });
    this.voice = new MockVoice(emit);
    this.seed();
  }

  private seed() {
    const { notes, revisions, threads } = createSeedData();
    for (const n of notes) this.notes.set(n.id, n);
    for (const [id, list] of revisions) this.revisions.set(id, list);
    for (const th of threads) this.threads.set(th.id, th);
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
  async listBacklinks(noteId: string) {
    return [...this.notes.values()]
      .filter((n) => n.id !== noteId && hasNoteLink(n.doc, noteId))
      .sort((a, b) => b.updatedAt - a.updatedAt)
      .map((n) => this.meta(n));
  }
  async getNote(id: string) {
    const n = this.requireNote(id);
    return structuredClone({ ...n, ...this.meta(n) });
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
  private recordRevision(noteId: string, titleText: string, doc: unknown, plain: string, ts: number, force = false) {
    const list = this.revisions.get(noteId) ?? [];
    const latest = list[0];
    if (latest && latest.title === titleText && latest.plain === plain) return;
    if (!force && latest && latest.createdAt > ts - REVISION_INTERVAL_MS) return;
    list.unshift({
      id: uid(),
      noteId,
      title: titleText,
      doc: structuredClone(doc),
      plain,
      createdAt: ts,
    });
    if (list.length > MAX_REVISIONS_PER_NOTE) list.length = MAX_REVISIONS_PER_NOTE;
    this.revisions.set(noteId, list);
  }

  async saveNote(id: string, titleText: string, doc: unknown, plain: string) {
    const n = this.requireNote(id);
    const ts = now();
    Object.assign(n, { title: titleText, doc, updatedAt: ts });
    if (plain.trim()) this.recordRevision(id, titleText, doc, plain, ts);
    return this.meta(n);
  }
  async listRevisions(noteId: string): Promise<NoteRevision[]> {
    return (this.revisions.get(noteId) ?? []).map(({ id, noteId: nid, title: t, plain, createdAt }) => ({
      id,
      noteId: nid,
      title: t,
      plain,
      createdAt,
    }));
  }
  async restoreRevision(noteId: string, revisionId: string): Promise<Note> {
    const n = this.requireNote(noteId);
    const list = this.revisions.get(noteId) ?? [];
    const target = list.find((r) => r.id === revisionId);
    if (!target) throw new Error(`revision not found: ${revisionId}`);
    const currentPlain = plainOf(n.doc);
    this.recordRevision(noteId, n.title, n.doc, currentPlain, now(), true);
    Object.assign(n, { title: target.title, doc: structuredClone(target.doc), updatedAt: now() });
    return this.getNote(noteId);
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
    this.ai.cancel(`${noteId}:chat`);
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
    return {
      providerId: p.id,
      hasStoredKey: !!stored,
      maskedKey: maskKey(stored),
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
    throw new Error(`Nothing answers at ${p?.baseUrl}`);
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
  private buildSearchStatus(): SearchStatus {
    const engines: SearchEngineCredentialStatus[] = SEARCH_ENGINES.map((spec) => {
      const stored = this.searchKeys.get(spec.engine);
      return {
        engine: spec.engine,
        label: spec.label,
        envVar: spec.envVar,
        docsUrl: spec.docsUrl,
        hasStoredKey: !!stored,
        maskedKey: maskKey(stored),
        hasEnvKey: false,
      };
    });
    const pinned = this.settings.webSearchEngine;
    if (pinned) {
      const entry = engines.find((e) => e.engine === pinned);
      if (entry && (entry.hasStoredKey || entry.hasEnvKey)) {
        return { available: true, engine: entry.engine, pinned, reason: `Using ${entry.label}`, engines };
      }
      const label = entry?.label ?? pinned;
      return { available: false, engine: null, pinned, reason: `${label} is selected, but its API key is missing`, engines };
    }
    const auto = engines.find((e) => e.hasStoredKey || e.hasEnvKey);
    if (auto) {
      return { available: true, engine: auto.engine, pinned: null, reason: `Using ${auto.label} (automatic)`, engines };
    }
    return { available: false, engine: null, pinned: null, reason: 'No search engine key set', engines };
  }
  async getSearchStatus() {
    return this.buildSearchStatus();
  }
  async setSearchKey(engine: SearchEngine, key: string) {
    const spec = SEARCH_ENGINES.find((e) => e.engine === engine);
    if (!spec) throw new Error(`unknown search engine "${engine}"`);
    const trimmed = key.trim();
    if (!trimmed) throw new Error('API key is empty');
    if (/\s/.test(trimmed)) throw new Error('API key cannot contain whitespace');
    if (spec.prefix && !trimmed.startsWith(spec.prefix)) {
      throw new Error(`${spec.label} keys must start with ${spec.prefix}`);
    }
    if (trimmed.length < spec.minLength) {
      throw new Error(`${spec.label} keys must be at least ${spec.minLength} characters`);
    }
    this.searchKeys.set(engine, trimmed);
    return this.buildSearchStatus();
  }
  async deleteSearchKey(engine: SearchEngine) {
    this.searchKeys.delete(engine);
    return this.buildSearchStatus();
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

  aiRead = (noteId: string, target: string) => this.ai.aiRead(noteId, target);
  aiAnswer = (noteId: string, target: string) => this.ai.aiAnswer(noteId, target);
  aiAsk = (noteId: string, target: string, question: string | null) => this.ai.aiAsk(noteId, target, question);
  aiVisualize = (noteId: string, target: string) => this.ai.aiVisualize(noteId, target);
  aiReply = (threadId: string, text: string) => this.ai.aiReply(threadId, text);
  aiChat = (noteId: string, text: string, near?: string) => this.ai.aiChat(noteId, text, near);
  aiCancelRead = (noteId: string) => this.ai.aiCancelRead(noteId);

  voiceStatus = () => this.voice.voiceStatus();
  voiceInstall = (kind: ModelKind) => this.voice.voiceInstall(kind);
  voiceRemove = (kind: ModelKind) => this.voice.voiceRemove(kind);
  listVoices = (refresh: boolean) => this.voice.listVoices(refresh);
  listListenModels = () => this.voice.listListenModels();
  getLocalWhisperState = () => this.voice.getLocalWhisperState();
  addWhisperModel = (path: string) => this.voice.addWhisperModel(path);
  removeWhisperModel = (path: string) => this.voice.removeWhisperModel(path);
  setWhisperBin = (path: string | null) => this.voice.setWhisperBin(path);
  listenStart = (handsFree: boolean) => this.voice.listenStart(handsFree);
  listenStop = () => this.voice.listenStop();
  speak = (messageId: string | null, text: string, voice?: string) => this.voice.speak(messageId, text, voice);
  stopSpeaking = () => this.voice.stopSpeaking();

  async exportNote(_id: string, _format: ExportFormat, _path: string) {}
  async importMarkdown(path: string, folderId: string | null = null) {
    const file = path.split(/[\\/]/).pop() ?? 'Imported note';
    const stem = file.replace(/\.(md|markdown|txt)$/i, '').trim() || 'Imported note';
    const note = await this.createNote(stem);
    const stored = this.requireNote(note.id);
    stored.doc = {
      type: 'doc',
      content: [title(stem), para(`Imported from ${file}.`)],
    };
    if (folderId && this.folders.has(folderId)) stored.folderId = folderId;
    return structuredClone(stored);
  }
  async exportMarkdownDir(_dir: string, folderId: string | null = null) {
    if (folderId === null) return this.notes.size;
    const allowed = new Set<string>([folderId]);
    let grew = true;
    while (grew) {
      grew = false;
      for (const f of this.folders.values()) {
        if (f.parentId && allowed.has(f.parentId) && !allowed.has(f.id)) {
          allowed.add(f.id);
          grew = true;
        }
      }
    }
    return [...this.notes.values()].filter((n) => n.folderId !== null && allowed.has(n.folderId)).length;
  }

  on<K extends keyof BackendEvents>(event: K, handler: (payload: BackendEvents[K]) => void): Unsubscribe {
    const set = (this.handlers.get(event) ?? new Set()) as Set<(payload: BackendEvents[K]) => void>;
    set.add(handler);
    this.handlers.set(event, set as Set<(payload: never) => void>);
    return () => set.delete(handler);
  }
}
