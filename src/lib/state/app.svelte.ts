import { tick } from 'svelte';
import { getBackend } from '../api/backend';
import { pickOpenPath, pickSavePath } from '../api/platform';
import type { DeleteMode, ExportFormat, Note, NoteMetaPatch } from '../api/types';
import { exposeDebug } from '../debug';
import { markdownish } from '../editor/plain';
import { notes, refreshNotes, removeMeta, upsertMeta } from './notes.svelte';
import { storeNote } from './saves';
import { loadSettings, settings, updateSettings } from './settings.svelte';
import { showToast } from './toast.svelte';
import { loadTrash } from './trash.svelte';

export type CustomizePage = 'general' | 'appearance' | 'providers' | 'keyboard' | 'minds' | 'about' | 'voice' | 'help';
/** `pinned` is the user's saved choice; `peek` came from hovering the left edge and is never saved. */
export type SidebarState = 'closed' | 'pinned' | 'peek';
/** The open note. Read its current title, folder and date from `notes.list`. */
export type OpenNote = Omit<Note, 'folderId' | 'updatedAt' | 'hasOpenQuestion'>;
/** A Mind form. `assign` gives a new Mind to the current note on save. */
export type MindEditor = { id: string } | { id: null; assign: boolean };

export const app = $state({
  note: null as OpenNote | null,
  ready: false,
  sidebar: 'closed' as SidebarState,
  get sidebarOpen() {
    return this.sidebar !== 'closed';
  },
  paletteOpen: false,
  chatOpen: false,
  trashOpen: false,
  historyOpen: false,
  customizeOpen: false,
  customizePage: 'general' as CustomizePage,
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
let openSeq = 0;

function show(note: Note) {
  openSeq++;
  const i = visited.indexOf(note.id);
  if (i >= 0) visited.splice(i, 1);
  visited.push(note.id);
  app.tellOpen = false;
  app.note = note;
}

export async function openNote(id: string) {
  if (app.note?.id === id) return;
  const seq = ++openSeq;
  await flushSave?.();
  if (seq !== openSeq) return;
  await storeNote(id);
  if (seq !== openSeq) return;
  const note = await (await getBackend()).getNote(id);
  if (seq !== openSeq) return;
  show(note);
}

export async function newNote(title = '') {
  await flushSave?.();
  const note = await (await getBackend()).createNote(title);
  upsertMeta(note);
  show(note);
  app.focusTitle = title === '';
}

/** To the note open before this one. */
async function leaveNote(id: string) {
  const left = new Set(notes.list.map((n) => n.id));
  left.delete(id);
  const next = visited.filter((v) => left.has(v)).at(-1) ?? [...left][0];
  if (next) await openNote(next);
  else await newNote();
}

/** `mode` is what the menu offered, so a setting that changes meanwhile never turns a trip to the trash into a loss. */
export async function deleteNote(id: string, mode: DeleteMode) {
  const wasOpen = app.note?.id === id;
  let movedTo: string | undefined;
  let went: DeleteMode;
  try {
    // What was typed in the note is stored first, which would fail once it is gone.
    if (wasOpen) {
      await leaveNote(id);
      movedTo = app.note?.id;
      // Its editor is gone a tick later, and hands over what was typed while the next note loaded.
      await tick();
    }
    await storeNote(id);
    went = await (await getBackend()).deleteNote(id, mode);
  } catch {
    await refreshNotes().catch(() => {});
    if (app.note?.id === id || notes.list.some((n) => n.id === id)) {
      showToast('That note couldn’t be deleted.');
      if (wasOpen) await openNote(id).catch(() => {});
      return;
    }
    went = mode === 'permanent' ? mode : 'duragon';
  }
  removeMeta(id);
  const undo = { label: 'Undo', run: () => void undoDelete(id, movedTo) };
  if (went === 'system') showToast('Moved to the system trash');
  else if (went === 'duragon') showToast(mode === 'system' ? 'The system trash wouldn’t take it. It’s in Duragon’s Trash.' : 'Moved to Trash', undo);
}

/** Reopens the note when the writer is still on the one that deleting it moved them to. */
async function undoDelete(id: string, movedTo: string | undefined) {
  if (!(await restoreNote(id)) || !movedTo || app.note?.id !== movedTo) return;
  app.trashOpen = false;
  await openNote(id);
}

/** Whether the note is back. */
export async function restoreNote(id: string): Promise<boolean> {
  let back = false;
  try {
    upsertMeta(await (await getBackend()).restoreNote(id));
    back = true;
  } catch {
    showToast('That note couldn’t be restored.');
  }
  if (app.trashOpen) await loadTrash();
  return back;
}

export async function restoreNoteRevision(revisionId: string) {
  if (!app.note) return;
  const noteId = app.note.id;
  try {
    await flushSave?.();
    await storeNote(noteId);
    const note = await (await getBackend()).restoreRevision(noteId, revisionId);
    show(note);
    upsertMeta(note);
    app.historyOpen = false;
    showToast('Restored earlier draft');
  } catch {
    showToast('That draft couldn’t be restored.');
  }
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

export function openCustomize(page: CustomizePage = 'general') {
  app.customizePage = page;
  app.customizeOpen = true;
}
/** From the note's Mind picker: only the form opens, over the note. */
export const newMindForNote = () => (app.mindEditor = { id: null, assign: true });

export const toggleSidebar = () => setSidebar(!app.sidebarOpen);
export const toggleChat = () => (app.chatOpen = !app.chatOpen);
export const toggleJustMyWords = () =>
  updateSettings({ justMyWords: !settings.value.justMyWords });

export const noteTitle = () => notes.list.find((n) => n.id === app.note?.id)?.title || 'Untitled';

export async function copyNote() {
  if (!app.note) return;
  await flushSave?.();
  const note = await (await getBackend()).getNote(app.note.id);
  await navigator.clipboard?.writeText(markdownish(note.doc));
}

const EXT: Record<ExportFormat, string> = { md: 'md', docx: 'docx' };

export async function exportNote(format: ExportFormat) {
  if (!app.note) return;
  const noteId = app.note.id;
  await flushSave?.();
  const path = await pickSavePath({
    defaultPath: `${noteTitle()}.${EXT[format]}`,
    filters: [{ name: format === 'md' ? 'Markdown' : 'Word', extensions: [EXT[format]] }],
  });
  if (path) await (await getBackend()).exportNote(noteId, format, path);
}

export async function importMarkdownNote(folderId: string | null = null) {
  await flushSave?.();
  const path = await pickOpenPath({
    filters: [{ name: 'Markdown', extensions: ['md', 'markdown', 'txt'] }],
    mockFallback: 'Imported note.md',
  });
  if (!path) return;
  try {
    const note = await (await getBackend()).importMarkdown(path, folderId);
    upsertMeta(note);
    show(note);
    showToast(`Imported “${note.title || 'Untitled'}”`);
  } catch {
    showToast('That file couldn’t be imported.');
  }
}

export async function exportAllMarkdown(folderId: string | null = null) {
  await flushSave?.();
  const dir = await pickOpenPath({
    directory: true,
    mockFallback: '/mock/export',
  });
  if (!dir) return;
  try {
    const count = await (await getBackend()).exportMarkdownDir(dir, folderId);
    showToast(count === 1 ? 'Exported 1 note' : `Exported ${count} notes`);
  } catch {
    showToast('Notes couldn’t be exported.');
  }
}

export const exportPdf = () => window.print();

exposeDebug('app', app);
exposeDebug('notes', notes);
