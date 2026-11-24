import { getBackend } from '../api/backend';
import { DEFAULT_PROVIDER_ID, DEFAULT_PROVIDERS, mergeProviders } from '../api/providers';
import type { ProviderCredentialStatus, Settings } from '../api/types';

const DEFAULTS: Settings = {
  newNoteMode: 'thinks', readingPace: 'regular', questionPlacement: 'both',
  webInNewNotes: false, voiceInNewNotes: false, voiceName: 'bm_george', speakingSpeed: 1,
  appearance: 'system', textSize: 'default', pageWidth: null, lettering: 'own',
  inkColour: 'ink', justMyWords: false, aboutYou: '',
  learnAboutYou: true, searchMyNotes: false, explainSymbols: false,
  readProvider: DEFAULT_PROVIDER_ID, chatProvider: DEFAULT_PROVIDER_ID, drawProvider: DEFAULT_PROVIDER_ID,
  readModel: 'sonnet', chatModel: 'sonnet', drawModel: 'opus',
  readThinking: 'default', chatThinking: 'default', drawThinking: 'default',
  providers: DEFAULT_PROVIDERS.map((p) => ({ ...p, args: [...p.args], models: [...p.models] })),
  sidebarOpen: false, deletedNotes: 'duragon',
};

export const PAGE_WIDTH_DEFAULT = 690;
export const PAGE_WIDTH_MIN = 520;
export const PAGE_WIDTH_MAX = 1100;

const dark = window.matchMedia('(prefers-color-scheme: dark)');

export const settings = $state<{ value: Settings; loaded: boolean }>({ value: { ...DEFAULTS }, loaded: false });
export const credentialStatuses = $state<{ value: ProviderCredentialStatus[] }>({ value: [] });

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
  credentialStatuses.value = await backend.listCredentialStatuses();
}

export async function updateSettings(patch: Partial<Settings>) {
  // In place, so only what reads a changed key re-runs; replacing the object re-ran every reader (~45 ms in WebKitGTK).
  Object.assign(settings.value, patch);
  apply();
  const backend = await getBackend();
  await backend.saveSettings($state.snapshot(settings.value));
  if (patch.providers) {
    credentialStatuses.value = await backend.listCredentialStatuses();
  }
}

export async function setProviderKey(providerId: string, key: string) {
  const updated = await (await getBackend()).setProviderKey(providerId, key);
  const idx = credentialStatuses.value.findIndex((c) => c.providerId === providerId);
  if (idx >= 0) credentialStatuses.value[idx] = updated;
  else credentialStatuses.value.push(updated);
}

export async function deleteProviderKey(providerId: string) {
  const updated = await (await getBackend()).deleteProviderKey(providerId);
  const idx = credentialStatuses.value.findIndex((c) => c.providerId === providerId);
  if (idx >= 0) credentialStatuses.value[idx] = updated;
  else credentialStatuses.value.push(updated);
}

if (import.meta.env.DEV) Object.assign(window as object, { __duragon: { settings, updateSettings } });
