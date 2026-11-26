<script lang="ts">
  import { onMount } from 'svelte';
  import { getBackend } from '../../api/backend';
  import type { ModelKind } from '../../api/types';
  import { refreshVoices, voice } from '../../state/voice.svelte';

  const MODELS: { kind: ModelKind; name: string; what: string; size: string }[] = [
    { kind: 'stt', name: 'Parakeet', what: 'Speech to text', size: '482 MB' },
    { kind: 'vad', name: 'Silero VAD', what: 'Hands-free listening', size: 'about 1 MB' },
    { kind: 'tts', name: 'Kokoro', what: 'Speech', size: '132 MB' },
  ];

  const st = $derived(voice.status);
  const busy = $derived(MODELS.some((m) => st[m.kind] === 'downloading'));
  const count = (n: number, one: string) => `${n} ${one}${n === 1 ? '' : 's'}`;
  const system = $derived.by(() => {
    const speechd = voice.voices.filter((v) => v.source === 'Speech Dispatcher');
    const whisper = voice.listenModels.filter((m) => m.engine === 'whisper.cpp');
    const modules = [...new Set(speechd.map((v) => v.engine))].join(', ');
    return [
      { name: 'Speech Dispatcher', what: 'System voices', found: speechd.length ? `${modules} · ${count(speechd.length, 'voice')}` : '' },
      { name: 'whisper.cpp', what: 'Speech to text', found: whisper.length ? count(whisper.length, 'model') : '' },
    ];
  });

  onMount(() => void refreshVoices(true));

  let failed = $state<string | null>(null);
  async function install(kind: ModelKind) {
    failed = null;
    try {
      await (await getBackend()).voiceInstall(kind);
    } catch (e) {
      failed = String(e);
    }
  }
  const error = $derived(st.error ?? failed);
</script>

<p class="intro">Duragon listens and speaks on this machine. Download any of its own models, or use the voices and models this computer already has. Nothing leaves your computer.</p>

<div class="list">
  {#each MODELS as m (m.kind)}
    {@const state = st[m.kind]}
    <div class="model">
      <div><div class="name">{m.name}</div><div class="what">{m.what}</div></div>
      {#if state === 'ready'}
        <div class="state ready">Installed</div>
      {:else if state === 'downloading'}
        <div class="state" role="progressbar" aria-label="Downloading {m.name}" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(st.progress * 100)}>
          Downloading {Math.round(st.progress * 100)}%
        </div>
      {:else}
        <button class="install" disabled={busy} aria-label="{state === 'error' ? 'Try again' : 'Install'} {m.name}" onclick={() => install(m.kind)}>
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

<h4>On this computer</h4>
<div class="list">
  {#each system as s (s.name)}
    <div class="model">
      <div><div class="name">{s.name}</div><div class="what">{s.what}</div></div>
      <div class="state" class:found={s.found}>{s.found || 'Not found'}</div>
    </div>
  {/each}
</div>

<style>
  .intro { max-width: 460px; font-size: 14px; line-height: 1.5; color: var(--graphite); }
  .list { margin-top: 22px; border: 1px solid var(--rule); border-radius: 12px; }
  .model { display: flex; align-items: center; justify-content: space-between; gap: 16px; min-height: 62px; padding: 12px 16px; border-top: 1px solid var(--rule); }
  .model:first-child { border-top: 0; }
  .name { font-size: 14px; font-weight: 500; }
  .what { margin-top: 1px; font-size: 13px; color: var(--faint); }
  .state { font-size: 13px; color: var(--faint); font-variant-numeric: tabular-nums; text-align: right; }
  .state.ready { color: var(--text-green); }
  .state.found { color: var(--graphite); }
  .bar { height: 2px; margin: -1px 16px 0; border-radius: 1px; background: var(--rule); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--ink); transform-origin: left; transition: transform 120ms linear; }
  .error { margin-top: 14px; font-size: 13.5px; color: var(--text-red); }
  .install { flex: none; height: 30px; padding: 0 14px; border-radius: 999px; background: var(--ink); color: var(--paper); font-size: 13px; transition: opacity 150ms var(--ease-ui), transform 160ms var(--ease-step); }
  .install .size { opacity: 0.7; }
  .install:disabled { opacity: 0.4; cursor: default; }
  h4 { margin: 30px 0 0; font-size: 13.5px; line-height: 1.3; font-weight: 500; }
  h4 + .list { margin-top: 10px; }
  /* Gives a little under the press. */
  .install:active:not(:disabled) { transform: scale(0.97); }
  @media (prefers-reduced-motion: reduce) { .install:active:not(:disabled) { transform: none; } }
</style>
