import { getBackend } from '../api/backend';
import type { ExportFormat, Note, NoteMetaPatch } from '../api/types';
import { markdownish } from '../editor/plain';
import { notes, refreshNotes, removeMeta, upsertMeta } from './notes.svelte';
import { loadSettings, settings, updateSettings } from './settings.svelte';

export type CustomizePage = 'customize' | 'minds' | 'about' | 'voice' | 'help';
/** `pinned` is the user's saved choice; `peek` came from hovering the left edge and is never saved. */
export type SidebarState = 'closed' | 'pinned' | 'peek';
/** A Mind form. `assign` gives a new Mind to the current note on save. */
export type MindEditor = { id: string } | { id: null; assign: boolean };

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
  mindEditor: null as MindEditor | null,
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

/** Notes in the order they were last open, the one open now last. */
const visited: string[] = [];

function show(note: Note) {
  const i = visited.indexOf(note.id);
  if (i >= 0) visited.splice(i, 1);
  visited.push(note.id);
  app.tellOpen = false;
  app.note = note;
}

export async function openNote(id: string) {
  if (app.note?.id === id) return;
  await flushSave?.();
  show(await (await getBackend()).getNote(id));
}

export async function newNote(title = '') {
  await flushSave?.();
  const note = await (await getBackend()).createNote(title);
  upsertMeta(note);
  show(note);
  app.focusTitle = title === '';
}

export async function deleteNote(id: string) {
  // Leaving the note saves what was just typed in it, which would fail once it is gone.
  if (app.note?.id === id) {
    const left = new Set(notes.list.map((n) => n.id));
    left.delete(id);
    const next = visited.filter((v) => left.has(v)).at(-1) ?? [...left][0];
    if (next) await openNote(next);
    else await newNote();
  }
  await (await getBackend()).deleteNote(id);
  removeMeta(id);
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
/** From the note's Mind picker: only the form opens, over the note. */
export const newMindForNote = () => (app.mindEditor = { id: null, assign: true });

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
