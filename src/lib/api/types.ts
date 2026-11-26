export type ReadingMode = 'thinks' | 'asks';
export type Tone = 'plain' | 'warm' | 'blunt' | 'riddle';
export type Frequency = 'rarely' | 'matters' | 'often';

export const BUILTIN_RIDDLER_ID = '00000000-0000-4000-8000-000000000001';

export interface NoteMeta {
  id: string;
  title: string;
  folderId: string | null;
  updatedAt: number;
  hasOpenQuestion: boolean;
}

export interface Note extends NoteMeta {
  doc: unknown;
  riddlerId: string;
  mode: ReadingMode;
  voiceOn: boolean;
  webOn: boolean;
  brief: string;
  createdAt: number;
}

export interface Folder {
  id: string;
  name: string;
  parentId: string | null;
  createdAt: number;
}

export type ThreadKind = 'question' | 'reply';
export type ThreadStatus = 'open' | 'resolved' | 'dismissed';
export type DrawingState = 'none' | 'pending' | 'done' | 'failed';

export interface Message {
  id: string;
  noteId: string;
  threadId: string | null;
  author: 'you' | 'riddled';
  text: string;
  drawing: DrawingState;
  svg: string | null;
  /** Pages the model looked at when Web was on; [] otherwise. */
  sources: Source[];
  /** What each symbol in its formulas stands for, when explainSymbols was on; [] otherwise. */
  symbols: MathSymbol[];
  /** The note was too long to send whole, so the Mind read it around the target or cursor. */
  trimmed: boolean;
  createdAt: number;
}

/** `tex` is the symbol exactly as written in a formula of the message. */
export interface MathSymbol {
  tex: string;
  meaning: string;
}

export interface Source {
  url: string;
  title: string;
}

export interface Thread {
  id: string;
  noteId: string;
  kind: ThreadKind;
  quote: string;
  status: ThreadStatus;
  pinned: boolean;
  createdAt: number;
  messages: Message[];
}

export interface Riddler {
  id: string;
  name: string;
  description: string;
  howItThinks: string;
  tone: Tone;
  frequency: Frequency;
  builtin: boolean;
}

export type ThinkingLevel = 'default' | 'low' | 'medium' | 'high' | 'xhigh';

export type ProviderKind =
  | 'claude-cli'
  | 'agy-acp'
  | 'codex-cli'
  | 'acp'
  | 'openai'
  | 'anthropic'
  | 'gemini'
  | 'ollama'
  | 'local';

export interface ProviderConfig {
  id: string;
  name: string;
  kind: ProviderKind;
  baseUrl?: string | null;
  command?: string | null;
  args: string[];
  envVar?: string | null;
  models: string[];
  builtin: boolean;
}

export interface ProviderCredentialStatus {
  providerId: string;
  hasStoredKey: boolean;
  maskedKey: string | null;
  envVar: string | null;
  hasEnvKey: boolean;
  /** Why the saved keys can't be read, for a provider that needs one. */
  error?: string | null;
}

export type SearchEngine = 'tavily' | 'brave' | 'exa' | 'firecrawl' | 'ollama';

export interface SearchEngineCredentialStatus {
  engine: SearchEngine;
  label: string;
  envVar: string;
  docsUrl: string;
  hasStoredKey: boolean;
  maskedKey: string | null;
  hasEnvKey: boolean;
  error?: string | null;
}

export interface SearchStatus {
  available: boolean;
  engine: SearchEngine | null;
  pinned: SearchEngine | null;
  reason: string;
  engines: SearchEngineCredentialStatus[];
}

export interface Settings {
  newNoteMode: ReadingMode;
  readingPace: 'regular' | 'slow';
  questionPlacement: 'both' | 'right' | 'marks';
  webInNewNotes: boolean;
  webSearchEngine: SearchEngine | null;
  voiceInNewNotes: boolean;
  /** `kokoro/<voice>` or `speechd/<module>/<name>`; a bare name is a Kokoro voice. */
  voiceName: string;
  speakingSpeed: number;
  /** `parakeet` or `whisper/<model path>`. */
  listenModel: string;
  appearance: 'system' | 'light' | 'dark';
  textSize: 'small' | 'default' | 'large';
  pageWidth: number | null;
  lettering: 'own' | 'notes';
  inkColour: 'ink' | 'blue' | 'green' | 'red' | 'purple' | 'brown' | 'orange';
  justMyWords: boolean;
  aboutYou: string;
  learnAboutYou: boolean;
  searchMyNotes: boolean;
  explainSymbols: boolean;
  readProvider: string;
  chatProvider: string;
  drawProvider: string;
  readModel: string;
  chatModel: string;
  drawModel: string;
  readThinking: ThinkingLevel;
  chatThinking: ThinkingLevel;
  drawThinking: ThinkingLevel;
  providers: ProviderConfig[];
  sidebarOpen: boolean;
  deletedNotes: DeleteMode;
}

/** Where a deleted note goes: Duragon's own trash, the system's, or nowhere. */
export type DeleteMode = 'duragon' | 'system' | 'permanent';

export interface TrashedNote {
  id: string;
  title: string;
  deletedAt: number;
}

export interface TrashList {
  /** Newest first. */
  notes: TrashedNote[];
  /** Files that are broken or from a newer Duragon, left as they are. */
  unreadable: number;
}

export interface NoteMetaPatch {
  riddlerId?: string;
  mode?: ReadingMode;
  voiceOn?: boolean;
  webOn?: boolean;
  brief?: string;
  folderId?: string | null;
}

export type AiTask = 'read' | 'answer' | 'reply' | 'chat' | 'ask' | 'visualize' | 'draw' | 'learn' | 'summarize';
export type ExportFormat = 'md' | 'docx';

export interface BackendEvents {
  'ai:thinking': { noteId: string; target: string; task: AiTask };
  'ai:thread': Thread;
  'ai:message': Message;
  'ai:quiet': { noteId: string; target: string };
  'ai:error': { noteId: string; task: AiTask; message: string };
  'about:updated': string;
  /** The side chat's summary of its older messages changed in the background. */
  'chat:summary': { noteId: string; summary: string };
  'voice:status': VoiceStatus;
  /** Microphone level 0..1 while listening, ~20 Hz. */
  'voice:level': number;
  /** Speech recognised; final=true once the utterance ends. */
  'voice:transcript': { text: string; final: boolean };
  /** Playback state of spoken messages. */
  'voice:speaking': { messageId: string | null; playing: boolean };
}

export type ModelState = 'missing' | 'downloading' | 'ready' | 'error';
export type ModelKind = 'stt' | 'vad' | 'tts';
export interface VoiceStatus {
  stt: ModelState;
  vad: ModelState;
  tts: ModelState;
  /** 0..1 while downloading. */
  progress: number;
  error: string | null;
  listening: boolean;
}
/** A voice to speak with, or a model to listen with. */
export interface VoiceOption {
  id: string;
  name: string;
  description: string;
  /** Kokoro, Pico, eSpeak NG, Parakeet, whisper.cpp… */
  engine: string;
  /** Duragon, Speech Dispatcher, on this computer. */
  source: string;
  /** Speech Dispatcher's language code. */
  language: string | null;
}
