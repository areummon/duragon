import { getBackend } from '../api/backend';
import type { Settings } from '../api/types';

const DEFAULTS: Settings = {
  newNoteMode: 'thinks', readingPace: 'regular', questionPlacement: 'both',
  webInNewNotes: false, voiceInNewNotes: false, voiceName: 'bm_george', speakingSpeed: 1,
  appearance: 'system', textSize: 'default', pageWidth: null, lettering: 'own',
  inkColour: 'ink', justMyWords: false, aboutYou: '',
  learnAboutYou: true, searchMyNotes: false, explainSymbols: false, readModel: 'sonnet', chatModel: 'sonnet', drawModel: 'opus',
  sidebarOpen: false, deletedNotes: 'duragon',
};

export const PAGE_WIDTH_DEFAULT = 690;
export const PAGE_WIDTH_MIN = 520;
export const PAGE_WIDTH_MAX = 1100;

const dark = window.matchMedia('(prefers-color-scheme: dark)');

export const settings = $state<{ value: Settings; loaded: boolean }>({ value: { ...DEFAULTS }, loaded: false });

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
  settings.value = { ...DEFAULTS, ...(await (await getBackend()).getSettings()) };
  settings.loaded = true;
  apply();
}

export async function updateSettings(patch: Partial<Settings>) {
  // In place, so only what reads a changed key re-runs; replacing the object re-ran every reader (~45 ms in WebKitGTK).
  Object.assign(settings.value, patch);
  apply();
  await (await getBackend()).saveSettings($state.snapshot(settings.value));
}

if (import.meta.env.DEV) Object.assign(window as object, { __duragon: { settings, updateSettings } });
