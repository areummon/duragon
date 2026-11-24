import { getBackend } from '../api/backend';
import type { TrashList } from '../api/types';
import { showToast } from './toast.svelte';

export const trash = $state<TrashList & { loaded: boolean }>({ notes: [], unreadable: 0, loaded: false });

/** Only the newest load is shown, whichever comes back last. */
let newest = 0;

export async function loadTrash() {
  const mine = ++newest;
  try {
    const list = await (await getBackend()).listTrash();
    if (mine === newest) Object.assign(trash, list, { loaded: true });
  } catch {
    if (mine === newest) showToast('The trash couldn’t be opened.');
  }
}

export function forgetTrash() {
  newest++;
  Object.assign(trash, { notes: [], unreadable: 0, loaded: false });
}

/** Deletes exactly these, so a note that arrives meanwhile stays. */
export async function deleteForever(ids: string[]) {
  try {
    await (await getBackend()).deleteTrashed(ids);
  } catch {
    showToast(ids.length === 1 ? 'That note couldn’t be deleted.' : 'Those notes couldn’t all be deleted.');
  }
  await loadTrash();
}
