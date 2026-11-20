import { getBackend } from '../api/backend';
import type { ExportFormat, Note, NoteMetaPatch } from '../api/types';
import { markdownish } from '../editor/plain';
import { notes, refreshNotes, removeMeta, upsertMeta } from './notes.svelte';
import { loadSettings, settings, updateSettings } from './settings.svelte';

export type CustomizePage = 'customize' | 'minds' | 'about' | 'voice' | 'help';
/** `pinned` is the user's saved choice; `peek` came from hovering the left edge and is never saved. */
export type SidebarState = 'closed' | 'pinned' | 'peek';

export const app = $state({
  note: null as Note | null,
  ready: false,
  sidebar: 'closed' as SidebarState,
  get sidebarOpen() {
    return this.sidebar !== 'closed';
  },
  paletteOpen: false,
  chatOpen: false,
  customizeOpen: false,
  customizePage: 'customize' as CustomizePage,
  /** Mind form: id null creates a new Mind. */
  mindEditor: null as { id: string | null } | null,
  tellOpen: false,
  focusTitle: false,
});

let flushSave: (() => Promise<void>) | null = null;
export function registerFlush(fn: (() => Promise<void>) | null) {
  flushSave = fn;
}

export async function initApp() {
  await Promise.all([loadSettings(), refreshNotes()]);
  app.sidebar = settings.value.sidebarOpen ? 'pinned' : 'closed';
  const first = notes.list[0];
  if (first) await openNote(first.id);
  else await newNote();
  app.ready = true;
}

export async function openNote(id: string) {
  if (app.note?.id === id) return;
  await flushSave?.();
  app.tellOpen = false;
  app.note = await (await getBackend()).getNote(id);
}

export async function newNote(title = '') {
  await flushSave?.();
  const note = await (await getBackend()).createNote(title);
  upsertMeta(note);
  app.tellOpen = false;
  app.note = note;
  app.focusTitle = title === '';
}

export async function deleteNote(id: string) {
  await (await getBackend()).deleteNote(id);
  removeMeta(id);
  if (app.note?.id !== id) return;
  app.note = null;
  const next = notes.list[0];
  if (next) await openNote(next.id);
  else await newNote();
}

export async function updateMeta(patch: NoteMetaPatch) {
  if (!app.note) return;
  const note = await (await getBackend()).updateNoteMeta(app.note.id, patch);
  if (app.note?.id === note.id) {
    const { doc: _doc, ...rest } = note;
    Object.assign(app.note, rest);
  }
  upsertMeta(note);
}

export async function moveNote(id: string, folderId: string | null) {
  const note = await (await getBackend()).updateNoteMeta(id, { folderId });
  if (app.note?.id === id) app.note.folderId = folderId;
  upsertMeta(note);
}

export function setSidebar(open: boolean) {
  const saved = app.sidebar === 'pinned';
  app.sidebar = open ? 'pinned' : 'closed';
  if (open !== saved) void updateSettings({ sidebarOpen: open });
}
export function peekSidebar() {
  if (app.sidebar === 'closed') app.sidebar = 'peek';
}
export function unpeekSidebar() {
  if (app.sidebar === 'peek') app.sidebar = 'closed';
}

export function openCustomize(page: CustomizePage = 'customize') {
  app.customizePage = page;
  app.customizeOpen = true;
}
export const openMindEditor = (id: string | null) => {
  openCustomize('minds');
  app.mindEditor = { id };
};

export const toggleSidebar = () => setSidebar(!app.sidebarOpen);
export const toggleChat = () => (app.chatOpen = !app.chatOpen);
export const toggleJustMyWords = () =>
  updateSettings({ justMyWords: !settings.value.justMyWords });

export async function copyNote() {
  if (!app.note) return;
  await flushSave?.();
  const note = await (await getBackend()).getNote(app.note.id);
  await navigator.clipboard?.writeText(markdownish(note.doc));
}

const EXT: Record<ExportFormat, string> = { md: 'md', docx: 'docx' };

export async function exportNote(format: ExportFormat) {
  if (!app.note) return;
  await flushSave?.();
  if (!('__TAURI_INTERNALS__' in window)) return;
  const { save } = await import('@tauri-apps/plugin-dialog');
  const path = await save({
    defaultPath: `${app.note.title || 'Untitled'}.${EXT[format]}`,
    filters: [{ name: format === 'md' ? 'Markdown' : 'Word', extensions: [EXT[format]] }],
  });
  if (path) await (await getBackend()).exportNote(app.note.id, format, path);
}

export const exportPdf = () => window.print();

if (import.meta.env.DEV) Object.assign((window as unknown as { __duragon: object }).__duragon, { app, notes });
