import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Backend, Unsubscribe } from './backend';
import type {
  BackendEvents, ExportFormat, Folder, Message, Note, NoteMeta, NoteMetaPatch,
  Riddler, Settings, Thread, ThreadKind, ThreadStatus, VoiceOption, VoiceStatus,
} from './types';

export class TauriBackend implements Backend {
  listNotes = () => invoke<NoteMeta[]>('list_notes');
  searchNotes = (query: string) => invoke<NoteMeta[]>('search_notes', { query });
  getNote = (id: string) => invoke<Note>('get_note', { id });
  createNote = (title?: string) => invoke<Note>('create_note', { title });
  saveNote = (id: string, title: string, doc: unknown, plain: string) =>
    invoke<NoteMeta>('save_note', { id, title, doc, plain });
  updateNoteMeta = (id: string, patch: NoteMetaPatch) =>
    invoke<Note>('update_note_meta', { id, patch });
  deleteNote = (id: string) => invoke<void>('delete_note', { id });

  listFolders = () => invoke<Folder[]>('list_folders');
  createFolder = (name: string) => invoke<Folder>('create_folder', { name });
  renameFolder = (id: string, name: string) => invoke<void>('rename_folder', { id, name });
  deleteFolder = (id: string) => invoke<void>('delete_folder', { id });

  listThreads = (noteId: string) => invoke<Thread[]>('list_threads', { noteId });
  setThreadStatus = (id: string, status: ThreadStatus) =>
    invoke<void>('set_thread_status', { id, status });
  setThreadPinned = (id: string, pinned: boolean) =>
    invoke<void>('set_thread_pinned', { id, pinned });
  setThreadKind = (id: string, kind: ThreadKind) => invoke<void>('set_thread_kind', { id, kind });
  threadToNote = (id: string) => invoke<Note>('thread_to_note', { id });
  listChat = (noteId: string) => invoke<Message[]>('list_chat', { noteId });

  getSettings = () => invoke<Settings>('get_settings');
  saveSettings = (settings: Settings) => invoke<void>('save_settings', { settings });
  listRiddlers = () => invoke<Riddler[]>('list_riddlers');
  saveRiddler = (riddler: Riddler) => invoke<Riddler>('save_riddler', { riddler });
  deleteRiddler = (id: string) => invoke<void>('delete_riddler', { id });

  aiRead = (noteId: string, target: string) => invoke<void>('ai_read', { noteId, target });
  aiAnswer = (noteId: string, target: string) => invoke<void>('ai_answer', { noteId, target });
  aiReply = (threadId: string, text: string) => invoke<void>('ai_reply', { threadId, text });
  aiChat = (noteId: string, text: string, near: string) => invoke<void>('ai_chat', { noteId, text, near });
  aiAsk = (noteId: string, target: string, question: string | null) =>
    invoke<void>('ai_ask', { noteId, target, question });
  aiVisualize = (noteId: string, target: string) => invoke<void>('ai_visualize', { noteId, target });
  aiCancelRead = (noteId: string) => invoke<void>('ai_cancel_read', { noteId });

  voiceStatus = () => invoke<VoiceStatus>('voice_status');
  voiceInstall = () => invoke<void>('voice_install');
  listVoices = () => invoke<VoiceOption[]>('list_voices');
  listenStart = (handsFree: boolean) => invoke<void>('listen_start', { handsFree });
  listenStop = () => invoke<string>('listen_stop');
  speak = (messageId: string | null, text: string) => invoke<void>('speak', { messageId, text });
  stopSpeaking = () => invoke<void>('stop_speaking');
  exportNote = (id: string, format: ExportFormat, path: string) =>
    invoke<void>('export_note', { id, format, path });

  on<K extends keyof BackendEvents>(
    event: K,
    handler: (payload: BackendEvents[K]) => void,
  ): Unsubscribe {
    const pending = listen<BackendEvents[K]>(event, (e) => handler(e.payload));
    return () => void pending.then((off) => off());
  }
}
