import { app } from './app.svelte';
import { notes } from './notes.svelte';

/** Name of the riddler on the open note. Falls back to the builtin's name. */
export const riddlerName = () => notes.riddlers.find((r) => r.id === app.note?.riddlerId)?.name ?? 'Duragon';

/** Names that address the assistant in a line: the riddler, Duragon, Riddled. */
export function addressNames(): string[] {
  return [...new Set([riddlerName(), 'Duragon', 'Riddled'])];
}
