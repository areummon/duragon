import { getBackend } from '../api/backend';
import type { VoiceOption, VoiceStatus } from '../api/types';
import { current } from '../editor/current';
import { app, openCustomize } from './app.svelte';
import { settings } from './settings.svelte';
import { showToast } from './toast.svelte';

const DOUBLE_TAP_MS = 350;
const BARS = 5;

export const voice = $state({
  status: { stt: 'missing', vad: 'missing', tts: 'missing', progress: 0, error: null, listening: false } as VoiceStatus,
  voices: [] as VoiceOption[],
  listenModels: [] as VoiceOption[],
  /** Last few microphone levels, newest last. */
  levels: Array<number>(BARS).fill(0),
  mode: 'off' as 'off' | 'hold' | 'hands',
  speakingId: null as string | null,
  playing: false,
});

/** A bare name from before Speech Dispatcher, such as `bm_george`, is a Kokoro voice. */
const voiceId = (name: string) => (name.includes('/') ? name : `kokoro/${name}`);
/** The voice in use: the chosen one, or the first there is when that one is gone. */
export const chosenVoice = (): VoiceOption | undefined =>
  voice.voices.find((v) => v.id === voiceId(settings.value.voiceName)) ?? voice.voices[0];
export const chosenListenModel = (): VoiceOption | undefined =>
  voice.listenModels.find((m) => m.id === settings.value.listenModel) ?? voice.listenModels[0];

/** `system` lists Speech Dispatcher's voices again. */
export async function refreshVoices(system: boolean) {
  const be = await getBackend();
  [voice.voices, voice.listenModels] = await Promise.all([be.listVoices(system), be.listListenModels()]);
}

let started = false;
const spoken = new Set<string>();

export async function initVoice() {
  if (started) return;
  started = true;
  const be = await getBackend();
  voice.status = await be.voiceStatus();
  await refreshVoices(false);
  be.on('voice:status', (s) => {
    const was = voice.status;
    voice.status = s;
    if (s.stt !== was.stt || s.tts !== was.tts) void refreshVoices(false);
  });
  be.on('voice:level', (l) => {
    voice.levels = [...voice.levels.slice(1), l];
  });
  be.on('voice:speaking', (p) => {
    voice.playing = p.playing;
    voice.speakingId = p.playing ? p.messageId : null;
  });
  be.on('voice:transcript', (t) => {
    if (voice.mode === 'hands' && t.final && t.text.trim()) insertParagraph(t.text.trim());
  });

  const autoSpeak = (noteId: string, id: string, author: string, text: string) => {
    if (author !== 'riddled' || spoken.has(id)) return;
    if (spoken.size >= 200) spoken.clear();
    spoken.add(id);
    if (app.note?.id === noteId && app.note.voiceOn) void readAloud(id, text);
  };
  be.on('ai:thread', (t) => t.messages.forEach((m) => autoSpeak(t.noteId, m.id, m.author, m.text)));
  be.on('ai:message', (m) => autoSpeak(m.noteId, m.id, m.author, m.text));
}

export async function readAloud(messageId: string | null, text: string, voiceId?: string) {
  if (!voice.voices.length) return needModel('Speaking needs a voice. Get one in Customize → Voice models.');
  try {
    await (await getBackend()).speak(messageId, text, voiceId);
  } catch (e) {
    showToast(String(e));
  }
}

export async function stopSpeaking() {
  await (await getBackend()).stopSpeaking();
}

const sampleId = (id: string) => `sample:${id}`;
export const samplePlaying = (id: string) => voice.playing && voice.speakingId === sampleId(id);

/** Plays a few words in a voice without choosing it, or stops them. */
export function toggleSample(id: string) {
  if (samplePlaying(id)) void stopSpeaking();
  else void readAloud(sampleId(id), 'Does this still hold?', id);
}

/** Typing stops playback. */
export function interruptSpeech() {
  if (voice.playing) void stopSpeaking();
}

function caretReady() {
  const ed = current.editor;
  if (ed?.state.selection.$from.parent.type.name === 'title') ed.commands.focus('end');
  return ed;
}

function insertWords(text: string) {
  const ed = caretReady();
  if (!ed || !text.trim()) return;
  const before = ed.state.selection.$from.nodeBefore?.textContent ?? '';
  const lead = before && !/\s$/.test(before) ? ' ' : '';
  ed.chain().focus().insertContent(lead + text.trim()).run();
}

function insertParagraph(text: string) {
  const ed = caretReady();
  if (!ed) return;
  const empty = ed.state.selection.$from.parent.content.size === 0;
  const chain = ed.chain().focus();
  (empty ? chain : chain.splitBlock()).insertContent(text).run();
}

let queue: Promise<unknown> = Promise.resolve();
const serial = (fn: () => Promise<void>) => (queue = queue.then(fn).catch(() => {}));

function begin(handsFree: boolean) {
  voice.mode = handsFree ? 'hands' : 'hold';
  voice.levels = Array<number>(BARS).fill(0);
  void serial(async () => void (await (await getBackend()).listenStart(handsFree)));
}

function end(insert: boolean) {
  const was = voice.mode;
  voice.mode = 'off';
  void serial(async () => {
    const text = await (await getBackend()).listenStop();
    if (insert && was === 'hold') insertWords(text);
  });
}

export const stopHandsFree = () => {
  if (voice.mode === 'hands') end(false);
};

let altDown = false;
let altUsed = false;
let lastDown = 0;
let swallow = false;
let holdSince = 0;
const MIN_HOLD_MS = 400;

function needModel(text: string) {
  showToast(text, {
    label: 'Open',
    run: () => openCustomize('voice'),
  });
}

export function voiceKeydown(e: KeyboardEvent) {
  if (e.key !== 'Alt') {
    if (altDown) {
      altUsed = true;
      if (voice.mode === 'hold') end(false);
    }
    return;
  }
  e.preventDefault();
  if (e.repeat || altDown) return;
  altDown = true;
  altUsed = false;
  swallow = false;
  const now = performance.now();
  const double = now - lastDown < DOUBLE_TAP_MS;
  lastDown = now;
  if (voice.mode === 'hands') {
    swallow = true;
    end(false);
    return;
  }
  if (!chosenListenModel()) {
    swallow = true;
    needModel('Speech to text needs a model. Get one in Customize → Voice models.');
    return;
  }
  if (double && voice.status.vad !== 'ready') {
    if (voice.mode === 'hold') end(false);
    swallow = true;
    needModel('Hands-free listening needs Silero VAD. Install it in Customize → Voice models.');
    return;
  }
  if (double) {
    if (voice.mode === 'hold') end(false);
    begin(true);
    swallow = true;
    lastDown = 0;
    return;
  }
  holdSince = now;
  begin(false);
}

export function voiceKeyup(e: KeyboardEvent) {
  if (e.key !== 'Alt') return;
  e.preventDefault();
  altDown = false;
  if (swallow) return;
  if (voice.mode === 'hold') end(!altUsed && performance.now() - holdSince >= MIN_HOLD_MS);
}

export function voiceBlur() {
  altDown = false;
  if (voice.mode === 'hold') end(false);
}

if (import.meta.env.DEV) Object.assign((window as unknown as { __duragon: object }).__duragon ?? {}, { voice });
