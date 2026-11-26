import type { BackendEvents, ModelKind, VoiceOption, VoiceStatus } from '../types';
import { KOKORO, PARAKEET, SPEECHD, WHISPER } from './fixtures';

type Emit = <K extends keyof BackendEvents>(event: K, payload: BackendEvents[K]) => void;

export class MockVoice {
  private voice: VoiceStatus = { stt: 'missing', vad: 'missing', tts: 'missing', progress: 0, error: null, listening: false };
  private levelTimer = 0;
  private transcriptTimer = 0;
  private speakTimer = 0;

  constructor(private emit: Emit) {}

  private setVoice(patch: Partial<VoiceStatus>) {
    this.voice = { ...this.voice, ...patch };
    this.emit('voice:status', { ...this.voice });
  }

  async voiceStatus(): Promise<VoiceStatus> {
    return { ...this.voice };
  }

  async voiceInstall(kind: ModelKind): Promise<void> {
    const v = this.voice;
    if (v.stt === 'downloading' || v.vad === 'downloading' || v.tts === 'downloading') return;
    this.setVoice({ [kind]: 'downloading', progress: 0, error: null });
    const start = performance.now();
    const ms = { stt: 3000, vad: 600, tts: 2000 }[kind];
    const tick = window.setInterval(() => {
      const p = Math.min(1, (performance.now() - start) / ms);
      if (p >= 1) {
        clearInterval(tick);
        this.setVoice({ [kind]: 'ready', progress: 1 });
      } else this.setVoice({ progress: p });
    }, 100);
  }

  async listVoices(_refresh: boolean): Promise<VoiceOption[]> {
    return [...(this.voice.tts === 'ready' ? KOKORO : []), ...SPEECHD].map((v) => ({ ...v }));
  }

  async listListenModels(): Promise<VoiceOption[]> {
    return [...(this.voice.stt === 'ready' ? [PARAKEET] : []), ...WHISPER].map((m) => ({ ...m }));
  }

  async listenStart(handsFree: boolean): Promise<void> {
    if (handsFree && this.voice.vad !== 'ready') {
      throw new Error('Hands-free listening needs Silero VAD. Install it in Customize → Voice models.');
    }
    this.setVoice({ listening: true });
    this.levelTimer = window.setInterval(() => this.emit('voice:level', Math.random() * 0.8 + 0.1), 50);
    this.transcriptTimer = window.setTimeout(() => {
      this.emit('voice:transcript', { text: 'This is what I said out loud.', final: true });
      if (handsFree) {
        this.transcriptTimer = window.setTimeout(() => {
          this.emit('voice:transcript', { text: 'And here is a second thought.', final: true });
        }, 2000);
      }
    }, 2000);
  }

  async listenStop(): Promise<string> {
    clearInterval(this.levelTimer);
    clearTimeout(this.transcriptTimer);
    this.emit('voice:level', 0);
    this.setVoice({ listening: false });
    return 'This is what I said out loud.';
  }

  async speak(messageId: string | null, _text: string, _voice?: string): Promise<void> {
    clearTimeout(this.speakTimer);
    // A real voice takes a moment to begin.
    this.speakTimer = window.setTimeout(() => {
      this.emit('voice:speaking', { messageId, playing: true });
      this.speakTimer = window.setTimeout(() => this.emit('voice:speaking', { messageId, playing: false }), 3000);
    }, 300);
  }

  async stopSpeaking(): Promise<void> {
    clearTimeout(this.speakTimer);
    this.emit('voice:speaking', { messageId: null, playing: false });
  }
}
