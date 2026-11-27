import type {
  BackendEvents, DeleteMode, ExportFormat, Folder, LocalWhisperState, Message, Note, NoteMeta, NoteMetaPatch, NoteRevision,
  ProviderCredentialStatus, Riddler, SearchEngine, SearchStatus, Settings, Thread, ThreadKind, ThreadStatus, TrashList, ModelKind, VoiceOption, VoiceStatus,
} from './types';

export type Unsubscribe = () => void;

export interface Backend {
  listNotes(): Promise<NoteMeta[]>;
  searchNotes(query: string): Promise<NoteMeta[]>;
  listBacklinks(noteId: string): Promise<NoteMeta[]>;
  getNote(id: string): Promise<Note>;
  createNote(title?: string): Promise<Note>;
  saveNote(id: string, title: string, doc: unknown, plain: string): Promise<NoteMeta>;
  listRevisions(noteId: string): Promise<NoteRevision[]>;
  restoreRevision(noteId: string, revisionId: string): Promise<Note>;
  updateNoteMeta(id: string, patch: NoteMetaPatch): Promise<Note>;
  /** Where the note went: 'duragon' when the system trash refused it. */
  deleteNote(id: string, mode: DeleteMode): Promise<DeleteMode>;

  listTrash(): Promise<TrashList>;
  /** Puts a note back with everything it had. One that is already live is returned as it is. */
  restoreNote(id: string): Promise<NoteMeta>;
  /** Deletes these notes from the trash for good, passing over any that is live. */
  deleteTrashed(ids: string[]): Promise<void>;

  listFolders(): Promise<Folder[]>;
  createFolder(name: string): Promise<Folder>;
  renameFolder(id: string, name: string): Promise<void>;
  moveFolder(id: string, parentId: string | null): Promise<void>;
  deleteFolder(id: string): Promise<void>;

  listThreads(noteId: string): Promise<Thread[]>;
  setThreadStatus(id: string, status: ThreadStatus): Promise<void>;
  setThreadPinned(id: string, pinned: boolean): Promise<void>;
  setThreadKind(id: string, kind: ThreadKind): Promise<void>;
  threadToNote(id: string): Promise<Note>;
  listChat(noteId: string): Promise<Message[]>;
  /** Deletes the side chat and its summary, cancelling a reply still on its way. */
  clearChat(noteId: string): Promise<void>;
  /** What the side chat remembers of its older messages, '' until it has folded any. */
  getChatSummary(noteId: string): Promise<string>;
  /** Saves the writer's edit of `base`, the summary they last saw; false when it has changed since. */
  saveChatSummary(noteId: string, base: string, text: string): Promise<boolean>;

  getSettings(): Promise<Settings>;
  saveSettings(settings: Settings): Promise<void>;
  listCredentialStatuses(): Promise<ProviderCredentialStatus[]>;
  /** The models a server the writer runs has now. Rejects when it doesn't answer. */
  localModels(providerId: string): Promise<string[]>;
  setProviderKey(providerId: string, key: string): Promise<ProviderCredentialStatus>;
  deleteProviderKey(providerId: string): Promise<ProviderCredentialStatus>;
  getSearchStatus(): Promise<SearchStatus>;
  setSearchKey(engine: SearchEngine, key: string): Promise<SearchStatus>;
  deleteSearchKey(engine: SearchEngine): Promise<SearchStatus>;
  listRiddlers(): Promise<Riddler[]>;
  saveRiddler(riddler: Riddler): Promise<Riddler>;
  deleteRiddler(id: string): Promise<void>;

  /** Reads `target`, the `nth` occurrence of its text in the note (from 0), so a long note is cut around that copy. */
  aiRead(noteId: string, target: string, nth: number): Promise<void>;
  /** Answers `target`, a line addressed to the mind and the `nth` occurrence of its text in the note (from 0). */
  aiAnswer(noteId: string, target: string, nth: number): Promise<void>;
  aiReply(threadId: string, text: string): Promise<void>;
  aiChat(noteId: string, text: string, near: string): Promise<void>;
  /** Selection actions. question=null is "{mind} this"; a string is "Ask {mind}". `nth` says which occurrence of the selected text was selected. */
  aiAsk(noteId: string, target: string, question: string | null, nth: number): Promise<void>;
  aiVisualize(noteId: string, target: string): Promise<void>;
  aiCancelRead(noteId: string): Promise<void>;

  voiceStatus(): Promise<VoiceStatus>;
  /** Download one model into the app data dir. Progress via voice:status. */
  voiceInstall(kind: ModelKind): Promise<void>;
  /** Remove a downloaded Duragon model from disk. */
  voiceRemove(kind: ModelKind): Promise<VoiceStatus>;
  /** Kokoro's voices when installed, then Speech Dispatcher's; `refresh` lists the system's again. */
  listVoices(refresh: boolean): Promise<VoiceOption[]>;
  listListenModels(): Promise<VoiceOption[]>;
  getLocalWhisperState(): Promise<LocalWhisperState>;
  addWhisperModel(path: string): Promise<LocalWhisperState>;
  removeWhisperModel(path: string): Promise<LocalWhisperState>;
  setWhisperBin(path: string | null): Promise<LocalWhisperState>;
  /** Start capturing the microphone; transcripts arrive as voice:transcript events. */
  listenStart(handsFree: boolean): Promise<void>;
  /** Stop capturing and return the final transcript of the utterance. */
  listenStop(): Promise<string>;
  /** With the chosen voice, or `voice` for a sample. */
  speak(messageId: string | null, text: string, voice?: string): Promise<void>;
  stopSpeaking(): Promise<void>;

  exportNote(id: string, format: ExportFormat, path: string): Promise<void>;
  importMarkdown(path: string, folderId?: string | null): Promise<Note>;
  exportMarkdownDir(dir: string, folderId?: string | null): Promise<number>;

  on<K extends keyof BackendEvents>(
    event: K,
    handler: (payload: BackendEvents[K]) => void,
  ): Unsubscribe;
}

let instance: Promise<Backend> | null = null;

export function getBackend(): Promise<Backend> {
  instance ??= '__TAURI_INTERNALS__' in window
    ? import('./tauri').then((m) => new m.TauriBackend())
    : import('./mock').then((m) => new m.MockBackend());
  return instance;
}
