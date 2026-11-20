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
  createdAt: number;
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

export interface Settings {
  newNoteMode: ReadingMode;
  readingPace: 'regular' | 'slow';
  questionPlacement: 'both' | 'right' | 'marks';
  webInNewNotes: boolean;
  voiceInNewNotes: boolean;
  voiceName: string;
  speakingSpeed: number;
  appearance: 'system' | 'light' | 'dark';
  textSize: 'small' | 'default' | 'large';
  pageWidth: number | null;
  lettering: 'own' | 'notes';
  inkColour: 'ink' | 'blue' | 'green' | 'red' | 'purple' | 'brown' | 'orange';
  inkFromPointer: boolean;
  justMyWords: boolean;
  aboutYou: string;
  learnAboutYou: boolean;
  readModel: string;
  chatModel: string;
  drawModel: string;
  sidebarOpen: boolean;
}

export interface NoteMetaPatch {
  riddlerId?: string;
  mode?: ReadingMode;
  voiceOn?: boolean;
  webOn?: boolean;
  brief?: string;
  folderId?: string | null;
}

export type AiTask = 'read' | 'answer' | 'reply' | 'chat' | 'ask' | 'visualize' | 'draw' | 'learn';
export type ExportFormat = 'md' | 'docx';

export interface BackendEvents {
  'ai:thinking': { noteId: string; target: string; task: AiTask };
  'ai:thread': Thread;
  'ai:message': Message;
  'ai:quiet': { noteId: string; target: string };
  'ai:error': { noteId: string; task: AiTask; message: string };
  'about:updated': string;
  'voice:status': VoiceStatus;
  /** Microphone level 0..1 while listening, ~20 Hz. */
  'voice:level': number;
  /** Speech recognised; final=true once the utterance ends. */
  'voice:transcript': { text: string; final: boolean };
  /** Playback state of spoken messages. */
  'voice:speaking': { messageId: string | null; playing: boolean };
}

export type ModelState = 'missing' | 'downloading' | 'ready' | 'error';
export interface VoiceStatus {
  stt: ModelState;
  tts: ModelState;
  /** 0..1 while downloading. */
  progress: number;
  error: string | null;
  listening: boolean;
}
export interface VoiceOption { id: string; name: string; description: string }
