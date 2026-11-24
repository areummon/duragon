import { getBackend } from '../api/backend';
import { docTitle, plainText } from '../editor/plain';
import { upsertMeta } from './notes.svelte';

const RETRY = 3000;

/** Each note's newest words, until they are stored. */
const waiting = new Map<string, unknown>();
/** The saves on their way, at most one per note, so an older one never lands on a newer one. */
const storing = new Map<string, Promise<void>>();
const retries = new Map<string, number>();

export function keepWords(id: string, doc: unknown) {
  waiting.set(id, doc);
}

/** Stores what waits for a note. Rejects when it can't be stored, and then keeps trying. */
export async function storeNote(id: string): Promise<void> {
  if (!waiting.has(id)) return;
  let run = storing.get(id);
  if (!run) storing.set(id, (run = store(id)));
  await run;
}

async function store(id: string) {
  window.clearTimeout(retries.get(id));
  retries.delete(id);
  try {
    const be = await getBackend();
    while (waiting.has(id)) {
      const doc = waiting.get(id);
      try {
        upsertMeta(await be.saveNote(id, docTitle(doc), doc, plainText(doc)));
      } catch (e) {
        if (waiting.get(id) === doc) throw e;
      }
      if (waiting.get(id) === doc) waiting.delete(id);
    }
  } catch (e) {
    retries.set(id, window.setTimeout(() => void storeNote(id).catch(() => {}), RETRY));
    throw e;
  } finally {
    storing.delete(id);
  }
}
