<script lang="ts">
  import { onMount } from 'svelte';
  import { getBackend, type Backend } from '../../api/backend';
  import { pickOpenPath } from '../../api/platform';
  import type { LocalWhisperState, ModelKind, Settings, VoiceOption } from '../../api/types';
  import { settings, updateSettings } from '../../state/settings.svelte';
  import { chosenListenModel, chosenVoice, refreshVoices, voice } from '../../state/voice.svelte';
  import SwitchRow from './SwitchRow.svelte';
  import VoiceRow from './VoiceRow.svelte';

  const MODELS: { kind: ModelKind; name: string; what: string; size: string }[] = [
    { kind: 'stt', name: 'Parakeet', what: 'Speech to text', size: '482 MB' },
    { kind: 'vad', name: 'Silero VAD', what: 'Hands-free listening', size: 'about 1 MB' },
    { kind: 'tts', name: 'Kokoro', what: 'Speech', size: '132 MB' },
  ];

  const s = $derived(settings.value);
  const set = (patch: Partial<Settings>) => void updateSettings(patch);

  const spoken = $derived(chosenVoice());
  const heard = $derived(chosenListenModel());
  const languages = new Intl.DisplayNames(undefined, { type: 'language' });
  function describeVoice(o: VoiceOption) {
    if (o.description || !o.language) return o.description;
    try {
      return languages.of(o.language) ?? o.language;
    } catch {
      return o.language;
    }
  }

  const st = $derived(voice.status);
  const busy = $derived(MODELS.some((m) => st[m.kind] === 'downloading'));
  const count = (n: number, one: string) => `${n} ${one}${n === 1 ? '' : 's'}`;
  const speechdSummary = $derived.by(() => {
    const speechd = voice.voices.filter((v) => v.source === 'Speech Dispatcher');
    const modules = [...new Set(speechd.map((v) => v.engine))].join(', ');
    return speechd.length ? `${modules} · ${count(speechd.length, 'voice')}` : '';
  });
  const whisperCount = $derived(voice.listenModels.filter((m) => m.engine === 'whisper.cpp').length);

  let localWhisper = $state<LocalWhisperState>({ binPath: null, binMissing: false, binCustom: false, models: [] });
  let localError = $state<string | null>(null);
  let removing = $state<ModelKind | null>(null);

  async function loadLocalWhisper() {
    try {
      localWhisper = await (await getBackend()).getLocalWhisperState();
    } catch (e) {
      localError = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(() => {
    void refreshVoices(true);
    void loadLocalWhisper();
  });

  let modelsEl = $state<HTMLElement>();
  const showModels = () => modelsEl?.scrollIntoView({ behavior: 'smooth', block: 'start' });

  let failed = $state<string | null>(null);
  async function install(kind: ModelKind) {
    failed = null;
    try {
      await (await getBackend()).voiceInstall(kind);
    } catch (e) {
      failed = String(e);
    }
  }

  async function removeBuiltin(kind: ModelKind) {
    if (busy || removing) return;
    failed = null;
    removing = kind;
    try {
      voice.status = await (await getBackend()).voiceRemove(kind);
      await refreshVoices(false);
    } catch (e) {
      failed = String(e);
    } finally {
      removing = null;
    }
  }

  async function localAction(f: (b: Backend) => Promise<LocalWhisperState>) {
    localError = null;
    try {
      localWhisper = await f(await getBackend());
      await refreshVoices(false);
    } catch (e) {
      localError = e instanceof Error ? e.message : String(e);
    }
  }

  async function addLocalWhisperModel() {
    const path = await pickOpenPath({
      filters: [{ name: 'whisper.cpp model', extensions: ['bin'] }],
      mockFallback: '/home/you/models/ggml-medium.en.bin',
    });
    if (path) await localAction((b) => b.addWhisperModel(path));
  }

  const removeLocalWhisperModel = (path: string) => localAction((b) => b.removeWhisperModel(path));

  async function pickWhisperBin() {
    const path = await pickOpenPath({
      mockFallback: '/usr/local/bin/whisper-cli',
    });
    if (path) await localAction((b) => b.setWhisperBin(path));
  }

  const clearWhisperBin = () => localAction((b) => b.setWhisperBin(null));

  const error = $derived(st.error ?? failed);
</script>

<section>
  <p class="intro">How Duragon sounds when it speaks, and what turns your voice into words.</p>

  <div class="group">
    <SwitchRow title="New notes speak aloud" desc="Each new thought is read out once as it’s written. Typing stops it." on={s.voiceInNewNotes} onchange={(v) => set({ voiceInNewNotes: v })} />
  </div>

  <div class="jobs">
    <VoiceRow label="Voice" id="voice" desc="Reads Duragon’s thoughts aloud." value={spoken?.name ?? 'None installed'} by={spoken?.engine ?? 'Install below'}
      placeholder="Find a voice" options={voice.voices} current={spoken?.id} describe={describeVoice} samples empty="No voices are installed on this computer."
      more="Get more voices" onpick={(id) => set({ voiceName: id })} onmore={showModels} />
    <VoiceRow label="Speech to text" id="listen" desc="Turns what you say into words." value={heard?.name ?? 'None installed'} by={heard ? (heard.engine === 'Parakeet' ? 'Duragon' : heard.engine) : 'Install below'}
      placeholder="Find a model" options={voice.listenModels} current={heard?.id} describe={(o) => o.description} empty="No speech-to-text models are installed on this computer."
      more="Get more models" onpick={(id) => set({ listenModel: id })} onmore={showModels} />
  </div>

  <div class="group">
    <div class="speedhead"><h4 id="g-speed">Speaking speed</h4><output class="val">{s.speakingSpeed.toFixed(2)}×</output></div>
    <input
      class="range"
      type="range"
      min="0.5"
      max="2"
      step="0.05"
      aria-labelledby="g-speed"
      value={s.speakingSpeed}
      style:--fill="{((s.speakingSpeed - 0.5) / 1.5) * 100}%"
      oninput={(e) => set({ speakingSpeed: Number(e.currentTarget.value) })}
      ondblclick={() => set({ speakingSpeed: 1 })}
    />
    <div class="ends"><span>Slower</span><span>Faster</span></div>
  </div>
</section>

<section id="voice-models" bind:this={modelsEl}>
  <h3>Models</h3>
  <p class="sub">Duragon listens and speaks on this machine. Download any of its own models, or use the voices and models this computer already has. Nothing leaves your computer.</p>

  <div class="list">
    {#each MODELS as m (m.kind)}
      {@const state = st[m.kind]}
      <div class="model">
        <div><div class="name">{m.name}</div><div class="what">{m.what}</div></div>
        {#if state === 'ready'}
          <div class="ready-actions">
            <span class="state ready">Installed</span>
            <button
              type="button"
              class="quiet"
              disabled={busy || removing !== null}
              aria-label="Remove {m.name}"
              onclick={() => removeBuiltin(m.kind)}
            >
              {removing === m.kind ? 'Removing…' : 'Remove'}
            </button>
          </div>
        {:else if state === 'downloading'}
          <div class="state" role="progressbar" aria-label="Downloading {m.name}" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(st.progress * 100)}>
            Downloading {Math.round(st.progress * 100)}%
          </div>
        {:else}
          <button class="install" disabled={busy || removing !== null} aria-label="{state === 'error' ? 'Try again' : 'Install'} {m.name}" onclick={() => install(m.kind)}>
            {state === 'error' ? 'Try again' : 'Install'} <span class="size">{m.size}</span>
          </button>
        {/if}
      </div>
      {#if state === 'downloading'}
        <div class="bar"><span style:transform="scaleX({st.progress})"></span></div>
      {/if}
    {/each}
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <h4 class="syshead">On this computer</h4>
  <div class="list">
    <div class="model">
      <div><div class="name">Speech Dispatcher</div><div class="what">System voices</div></div>
      <div class="state" class:found={Boolean(speechdSummary)}>{speechdSummary || 'Not found'}</div>
    </div>

    <div class="model">
      <div>
        <div class="name">whisper.cpp</div>
        <div class="what">
          {#if whisperCount > 0}
            {count(whisperCount, 'model')} ready
          {:else if localWhisper.binPath && !localWhisper.binMissing}
            Binary ready · add a .bin model
          {:else}
            Speech to text · local GGML models
          {/if}
        </div>
      </div>
      <div class="local-actions">
        <button type="button" class="chip" onclick={addLocalWhisperModel}>Add model…</button>
        <button type="button" class="quiet" onclick={pickWhisperBin}>
          {localWhisper.binCustom ? 'Change binary…' : 'Set binary…'}
        </button>
      </div>
    </div>

    {#if localWhisper.binCustom && localWhisper.binPath}
      <div class="subrow">
        <div class="subtext">
          <span class="subname">whisper-cli binary</span>
          <span class="subpath" title={localWhisper.binPath}>{localWhisper.binPath}</span>
        </div>
        <div class="ready-actions">
          {#if localWhisper.binMissing}
            <span class="badge missing">Missing</span>
          {:else}
            <span class="badge ready">Ready</span>
          {/if}
          <button type="button" class="quiet" onclick={clearWhisperBin}>Reset</button>
        </div>
      </div>
    {/if}

    {#each localWhisper.models as item (item.path)}
      <div class="subrow">
        <div class="subtext">
          <span class="subname">{item.name}</span>
          <span class="subpath" title={item.path}>{item.path}</span>
        </div>
        <div class="ready-actions">
          {#if item.missing}
            <span class="badge missing">Missing</span>
          {:else}
            <span class="badge ready">Ready</span>
          {/if}
          <button type="button" class="quiet" aria-label="Remove {item.name}" onclick={() => removeLocalWhisperModel(item.path)}>Remove</button>
        </div>
      </div>
    {/each}
  </div>

  {#if localError}<p class="error" role="alert">{localError}</p>{/if}
  <p class="footnote">
    Add any local <code>whisper.cpp</code> <code>.bin</code> model from disk.
  </p>
</section>

<style>
  section { margin-top: 34px; }
  section:first-child { margin-top: 0; }
  .intro { max-width: 460px; font-size: 14px; line-height: 1.5; color: var(--graphite); }
  h3 { margin: 0; font-size: 16px; line-height: 1.3; font-weight: 500; }
  .sub { max-width: 460px; margin-top: 3px; font-size: 13.5px; line-height: 1.45; color: var(--faint); }
  .group { margin-top: 22px; }
  h4 { margin: 0 0 10px; font-size: 13.5px; line-height: 1.3; font-weight: 500; color: var(--ink); }

  .jobs { margin-top: 22px; border: 1px solid var(--rule); border-radius: 12px; }

  .speedhead { display: flex; justify-content: space-between; align-items: baseline; }
  .speedhead h4 { margin-bottom: 8px; }
  .val { font-size: 13px; color: var(--graphite); font-variant-numeric: tabular-nums; }
  .range { width: 100%; margin: 4px 0 0; height: 20px; background: transparent; appearance: none; cursor: var(--cursor-pointer); }
  .range::-webkit-slider-runnable-track { height: 2px; border-radius: 1px; background: linear-gradient(to right, var(--ink) var(--fill), var(--rule) var(--fill)); }
  .range::-webkit-slider-thumb { appearance: none; width: 16px; height: 16px; margin-top: -7px; border-radius: 50%; background: var(--paper); border: 1.5px solid var(--ink); }
  .range::-moz-range-track { height: 2px; background: var(--rule); }
  .range::-moz-range-progress { height: 2px; background: var(--ink); }
  .range::-moz-range-thumb { width: 13px; height: 13px; border-radius: 50%; background: var(--paper); border: 1.5px solid var(--ink); }
  .ends { display: flex; justify-content: space-between; margin-top: 2px; font-size: 12.5px; color: var(--faint); }

  .list { margin-top: 18px; border: 1px solid var(--rule); border-radius: 12px; }
  .model { display: flex; align-items: center; justify-content: space-between; gap: 16px; min-height: 62px; padding: 12px 16px; border-top: 1px solid var(--rule); }
  .model:first-child { border-top: 0; }
  .subrow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 9px 16px 9px 24px;
    border-top: 1px solid var(--rule);
    background: color-mix(in srgb, var(--shelf) 55%, transparent);
  }
  .subrow:last-child { border-bottom-left-radius: 11px; border-bottom-right-radius: 11px; }
  .subtext { min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .subname { font-size: 13px; line-height: 1.3; font-weight: 500; color: var(--ink); }
  .subpath { font-size: 12px; line-height: 1.3; color: var(--faint); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .name { font-size: 14px; font-weight: 500; }
  .what { margin-top: 1px; font-size: 13px; color: var(--faint); }
  .state { font-size: 13px; color: var(--faint); font-variant-numeric: tabular-nums; text-align: right; }
  .state.ready { color: var(--text-green); }
  .state.found { color: var(--graphite); }
  .ready-actions, .local-actions { display: inline-flex; align-items: center; gap: 10px; flex: none; }
  .badge { font-size: 12px; line-height: 1.3; font-weight: 500; }
  .badge.ready { color: var(--text-green); }
  .badge.missing { color: var(--text-red); }
  .chip {
    flex: none;
    height: 28px;
    padding: 0 11px;
    border: 1px solid var(--rule);
    border-radius: 8px;
    background: var(--paper);
    color: var(--ink);
    font-size: 12.5px;
    line-height: 1;
    text-box: trim-both cap alphabetic;
    transition: background-color var(--dur-ui) var(--ease-ui), border-color var(--dur-ui) var(--ease-ui);
  }
  @media (hover: hover) and (pointer: fine) {
    .chip:hover {
      background: var(--shelf);
      border-color: color-mix(in srgb, var(--ink) 20%, var(--rule));
    }
  }
  .quiet {
    flex: none;
    font-size: 12.5px;
    line-height: 1.3;
    color: var(--faint);
    transition: color var(--dur-ui) var(--ease-ui);
  }
  .quiet:hover:not(:disabled) { color: var(--ink); }
  .quiet:disabled { opacity: 0.4; cursor: default; }
  .bar { height: 2px; margin: -1px 16px 0; border-radius: 1px; background: var(--rule); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--ink); transform-origin: left; transition: transform 120ms linear; }
  .error { margin-top: 12px; font-size: 13px; line-height: 1.4; color: var(--text-red); }
  .footnote { margin-top: 10px; max-width: 520px; font-size: 12.5px; line-height: 1.45; color: var(--faint); }
  .footnote code { font-family: var(--font-mono); font-size: 11.5px; }
  .install { flex: none; height: 30px; padding: 0 14px; border-radius: 999px; background: var(--ink); color: var(--paper); font-size: 13px; transition: opacity 150ms var(--ease-ui), transform 160ms var(--ease-step); }
  .install .size { opacity: 0.7; }
  .install:disabled { opacity: 0.4; cursor: default; }
  .syshead { margin: 26px 0 0; }
  .syshead + .list { margin-top: 10px; }
  /* Gives a little under the press. */
  .install:active:not(:disabled) { transform: scale(0.97); }
  @media (prefers-reduced-motion: reduce) { .install:active:not(:disabled) { transform: none; } }
</style>
