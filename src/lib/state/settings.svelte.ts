import { getBackend } from '../api/backend';
import { DEFAULT_MODELS, DEFAULT_PROVIDER_ID, DEFAULT_PROVIDERS, mergeProviders, withoutProvider } from '../api/providers';
import type { ProviderCredentialStatus, Settings } from '../api/types';

const DEFAULTS: Settings = {
  newNoteMode: 'thinks', readingPace: 'regular', questionPlacement: 'both',
  webInNewNotes: false, voiceInNewNotes: false, voiceName: 'bm_george', speakingSpeed: 1,
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

function apply() {
  const s = settings.value;
  const root = document.documentElement;
  root.dataset.theme = s.appearance === 'system' ? (dark.matches ? 'dark' : 'light') : s.appearance;
  if (s.inkColour === 'ink') delete root.dataset.ink;
  else root.dataset.ink = s.inkColour;
  root.dataset.lettering = s.lettering;
  root.dataset.textSize = s.textSize;
  root.dataset.justMyWords = String(s.justMyWords);
}

dark.addEventListener('change', apply);

export async function loadSettings() {
  const backend = await getBackend();
  const loaded = await backend.getSettings();
  settings.value = {
    ...DEFAULTS,
    ...loaded,
    providers: mergeProviders(loaded.providers),
  };
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

let saving: Promise<unknown> = Promise.resolve();

export async function updateSettings(patch: Partial<Settings>) {
  // In place, so only what reads a changed key re-runs; replacing the object re-ran every reader (~45 ms in WebKitGTK).
  Object.assign(settings.value, patch);
  apply();
  const backend = await getBackend();
  const save = backend.saveSettings($state.snapshot(settings.value));
  saving = save.catch(() => {});
  await save;
  if (patch.providers) await refreshCredentials();
}

/** Stored before it shows. The core keeps everything as it was when the provider's key can't be forgotten with it. */
export async function removeProvider(id: string) {
  await saving;
  const patch = withoutProvider(settings.value, id);
  await (await getBackend()).saveSettings($state.snapshot({ ...settings.value, ...patch }) as Settings);
  Object.assign(settings.value, patch);
  await refreshCredentials();
}

export async function setProviderKey(providerId: string, key: string) {
  const backend = await getBackend();
  // The core takes a key only for a provider it has stored.
  await saving;
  await backend.setProviderKey(providerId, key);
  await refreshCredentials();
}

export async function deleteProviderKey(providerId: string) {
  await (await getBackend()).deleteProviderKey(providerId);
  await refreshCredentials();
}

if (import.meta.env.DEV) Object.assign(window as object, { __duragon: { settings, updateSettings } });
