import { getBackend } from '../api/backend';
import type { TrashList } from '../api/types';
import { showToast } from './toast.svelte';

export const trash = $state<TrashList & { loaded: boolean }>({ notes: [], unreadable: 0, loaded: false });

export async function loadTrash() {
  try {
    Object.assign(trash, await (await getBackend()).listTrash());
    trash.loaded = true;
  } catch {
    showToast('The trash couldn’t be opened.');
  }
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
