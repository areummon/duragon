import { getBackend } from '../api/backend';
import { DEFAULT_MODELS, DEFAULT_PROVIDER_ID, DEFAULT_PROVIDERS, isServerKind, mergeProviders, withoutProvider, type Server } from '../api/providers';
import type { ProviderCredentialStatus, Settings } from '../api/types';
import { showToast } from './toast.svelte';

const DEFAULTS: Settings = {
  newNoteMode: 'thinks', readingPace: 'regular', questionPlacement: 'both',
  webInNewNotes: false, voiceInNewNotes: false, voiceName: 'kokoro/bm_george', speakingSpeed: 1, listenModel: 'parakeet',
  appearance: 'system', textSize: 'default', pageWidth: null, lettering: 'own',
  inkColour: 'ink', justMyWords: false, aboutYou: '',
  learnAboutYou: true, searchMyNotes: false, explainSymbols: false,
  readProvider: DEFAULT_PROVIDER_ID, chatProvider: DEFAULT_PROVIDER_ID, drawProvider: DEFAULT_PROVIDER_ID,
  ...DEFAULT_MODELS,
  readThinking: 'default', chatThinking: 'default', drawThinking: 'default',
  providers: DEFAULT_PROVIDERS.map((p) => ({ ...p, args: [...p.args], models: [...p.models] })),
  sidebarOpen: false, deletedNotes: 'duragon',
};

export const PAGE_WIDTH_DEFAULT = 690;
export const PAGE_WIDTH_MIN = 520;
export const PAGE_WIDTH_MAX = 1100;

const dark = window.matchMedia('(prefers-color-scheme: dark)');

export const settings = $state<{ value: Settings; loaded: boolean }>({ value: { ...DEFAULTS }, loaded: false });
/** `error` is why the saved keys can't be read. */
export const credentialStatuses = $state<{ value: ProviderCredentialStatus[]; error: string | null }>({ value: [], error: null });

/** Fades the whole window from one theme to the other, in one picture. Colours that each fade restyle every frame. */
function showTheme(theme: 'light' | 'dark') {
  const root = document.documentElement;
  const shown = root.dataset.theme;
  if (shown === theme) return;
  const show = () => {
    // Or every button fades its own colours beneath the one fade.
    root.dataset.switching = '';
    root.dataset.theme = theme;
    void root.offsetHeight;
    delete root.dataset.switching;
  };
  if (!shown || !document.startViewTransition) return show();
  const { ready, finished } = document.startViewTransition(show);
  // A newer change, or a window out of sight, ends it early.
  ready.catch(() => {});
  finished.catch(() => {});
}

function apply() {
  const s = settings.value;
  const root = document.documentElement;
  showTheme(s.appearance === 'system' ? (dark.matches ? 'dark' : 'light') : s.appearance);
  if (s.inkColour === 'ink') delete root.dataset.ink;
  else root.dataset.ink = s.inkColour;
  root.dataset.lettering = s.lettering;
  root.dataset.textSize = s.textSize;
  root.dataset.justMyWords = String(s.justMyWords);
}

dark.addEventListener('change', apply);

async function storedSettings(): Promise<Settings> {
  const loaded = await (await getBackend()).getSettings();
  return { ...DEFAULTS, ...loaded, providers: mergeProviders(loaded.providers) };
}

export async function loadSettings() {
  // Every save sends all the settings, so one made after the core learns something has to have it too.
  (await getBackend()).on('about:updated', (text) => {
    if (!ownWords) settings.value.aboutYou = text;
  });
  settings.value = await storedSettings();
  settings.loaded = true;
  apply();
  await refreshCredentials();
}

/** What went wrong, to end a sentence with. */
export const reasonOf = (e: unknown) => (e instanceof Error ? e.message : String(e)).trim().replace(/\.+$/, '');

async function refreshCredentials() {
  try {
    credentialStatuses.value = await (await getBackend()).listCredentialStatuses();
    const unreadable = credentialStatuses.value.find((c) => c.error)?.error;
    credentialStatuses.error = unreadable ? reasonOf(unreadable) : null;
  } catch (e) {
    credentialStatuses.error = reasonOf(e);
  }
}

export const servers = $state<Record<string, Server>>({});
const checks: Record<string, number> = {};

/** Asks the writer's model servers what they have. */
export async function checkServers(only?: string) {
  const backend = await getBackend();
  // The core asks the address it has stored.
  await last;
  const asked = settings.value.providers.filter((p) => isServerKind(p.kind) && (!only || p.id === only));
  await Promise.all(asked.map(async ({ id }) => {
    const check = (checks[id] = (checks[id] ?? 0) + 1);
    servers[id] ??= { kind: 'checking' };
    const found = await backend.localModels(id).then(
      (models): Server => ({ kind: 'running', models: [...new Set(models)] }),
      (e): Server => ({ kind: 'stopped', reason: reasonOf(e) }),
    );
    // An older check can take longer than the one after it.
    if (check === checks[id]) servers[id] = found;
  }));
}

let last: Promise<unknown> = Promise.resolve();

/** Saves and key changes run one at a time, in order, whether or not the one before worked. */
function inTurn<T>(act: () => Promise<T>): Promise<T> {
  const run = last.then(act);
  last = run.catch(() => {});
  return run;
}

/** What the core says when the writer answers no to a new command or address. */
const NOT_ALLOWED = 'The change was not allowed';

let edits = 0;
/** Saves of the writer's own paragraph on their way. They replace what the core learned before they get there. */
let ownWords = 0;

/** Whether the change was stored. If not, the writer has been told, and what is stored shows again. */
export function updateSettings(patch: Partial<Settings>): Promise<boolean> {
  // In place, so only what reads a changed key re-runs; replacing the object re-ran every reader (~45 ms in WebKitGTK).
  Object.assign(settings.value, patch);
  apply();
  const edit = ++edits;
  const saved = inTurn(async () => {
    try {
      await (await getBackend()).saveSettings($state.snapshot(settings.value));
    } catch (e) {
      const stored = await storedSettings().catch(() => null);
      // A newer change is saved next, and this one with it.
      if (edit !== edits) return false;
      if (stored) Object.assign(settings.value, stored);
      apply();
      showToast(reasonOf(e) === NOT_ALLOWED ? 'That wasn’t allowed, so it wasn’t saved.' : 'That change couldn’t be saved.');
      return false;
    }
    if (patch.providers) {
      await refreshCredentials();
      void checkServers();
    }
    return true;
  });
  if ('aboutYou' in patch) {
    ownWords++;
    void saved.finally(() => ownWords--);
  }
  return saved;
}

/** Stored before it shows. The core keeps everything as it was when the provider's key can't be forgotten with it. */
export function removeProvider(id: string) {
  return inTurn(async () => {
    const without = { ...settings.value, ...withoutProvider(settings.value, id) };
    await (await getBackend()).saveSettings($state.snapshot(without) as Settings);
    // Worked out again, to keep an edit made meanwhile.
    Object.assign(settings.value, withoutProvider(settings.value, id));
    await refreshCredentials();
  });
}

// The core takes a key only for a provider it has stored.
export function setProviderKey(providerId: string, key: string) {
  return inTurn(async () => {
    await (await getBackend()).setProviderKey(providerId, key);
    await refreshCredentials();
    void checkServers(providerId);
  });
}

export function deleteProviderKey(providerId: string) {
  return inTurn(async () => {
    await (await getBackend()).deleteProviderKey(providerId);
    await refreshCredentials();
    void checkServers(providerId);
  });
}

if (import.meta.env.DEV) Object.assign(window as object, { __duragon: { settings, updateSettings } });
