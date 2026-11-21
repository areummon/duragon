import { getBackend } from '../api/backend';
import type { Folder, NoteMeta, Riddler } from '../api/types';

export const notes = $state<{ list: NoteMeta[]; folders: Folder[]; riddlers: Riddler[] }>({
  list: [], folders: [], riddlers: [],
});

const byNewest = (a: NoteMeta, b: NoteMeta) => b.updatedAt - a.updatedAt;

export async function refreshNotes() {
  const be = await getBackend();
  const [list, folders, riddlers] = await Promise.all([be.listNotes(), be.listFolders(), be.listRiddlers()]);
  notes.list = list.sort(byNewest);
  notes.folders = folders;
  notes.riddlers = riddlers;
}

export function upsertMeta(m: NoteMeta) {
  const meta: NoteMeta = {
    id: m.id, title: m.title, folderId: m.folderId, updatedAt: m.updatedAt, hasOpenQuestion: m.hasOpenQuestion,
  };
  notes.list = [...notes.list.filter((n) => n.id !== meta.id), meta].sort(byNewest);
}

export function removeMeta(id: string) {
  notes.list = notes.list.filter((n) => n.id !== id);
}

export async function createFolder(name: string) {
  const f = await (await getBackend()).createFolder(name);
  notes.folders = [...notes.folders, f];
}

export async function renameFolder(id: string, name: string) {
  await (await getBackend()).renameFolder(id, name);
  notes.folders = notes.folders.map((f) => (f.id === id ? { ...f, name } : f));
}

/** True when `id` is `ancestor` or sits somewhere inside it. */
export function folderWithin(id: string | null, ancestor: string): boolean {
  for (let at = id; at !== null; at = notes.folders.find((f) => f.id === at)?.parentId ?? null) {
    if (at === ancestor) return true;
  }
  return false;
}

export async function moveFolder(id: string, parentId: string | null) {
  if (parentId !== null && folderWithin(parentId, id)) return;
  await (await getBackend()).moveFolder(id, parentId);
  notes.folders = notes.folders.map((f) => (f.id === id ? { ...f, parentId } : f));
}

export async function deleteFolder(id: string) {
  const up = notes.folders.find((f) => f.id === id)?.parentId ?? null;
  await (await getBackend()).deleteFolder(id);
  notes.folders = notes.folders.filter((f) => f.id !== id).map((f) => (f.parentId === id ? { ...f, parentId: up } : f));
  notes.list = notes.list.map((n) => (n.folderId === id ? { ...n, folderId: up } : n));
}

export async function saveMind(r: Riddler): Promise<Riddler> {
  const saved = await (await getBackend()).saveRiddler(r);
  notes.riddlers = notes.riddlers.some((x) => x.id === saved.id)
    ? notes.riddlers.map((x) => (x.id === saved.id ? saved : x))
    : [...notes.riddlers, saved];
  return saved;
}

export async function removeMind(id: string) {
  await (await getBackend()).deleteRiddler(id);
  notes.riddlers = notes.riddlers.filter((x) => x.id !== id);
}
