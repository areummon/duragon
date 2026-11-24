<script lang="ts">
  import { getBackend } from '../../api/backend';
  import type { ModelState } from '../../api/types';
  import { voice } from '../../state/voice.svelte';

  const LABEL: Record<ModelState, string> = { missing: 'Not installed', downloading: 'Downloading', ready: 'Ready', error: 'Failed' };
  const st = $derived(voice.status);
  const busy = $derived(st.stt === 'downloading' || st.tts === 'downloading');
  const done = $derived(st.stt === 'ready' && st.tts === 'ready');
  const models = $derived([
    { name: 'Parakeet', what: 'Speech to text', state: st.stt },
    { name: 'Kokoro', what: 'Speech', state: st.tts },
  ]);

  let failed = $state<string | null>(null);
  async function install() {
    failed = null;
    try {
      await (await getBackend()).voiceInstall();
    } catch (e) {
      failed = String(e);
    }
  }
  const error = $derived(st.error ?? failed);
</script>

<p class="intro">Duragon listens and speaks on this machine, with two small models. They download once and nothing leaves your computer.</p>

<div class="list">
  {#each models as m (m.name)}
    <div class="model">
      <div><div class="name">{m.name}</div><div class="what">{m.what}</div></div>
      <div class="state" class:ready={m.state === 'ready'} class:err={m.state === 'error'}>{LABEL[m.state]}</div>
    </div>
  {/each}
</div>

{#if busy || (done && st.progress > 0 && st.progress < 1)}
  <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(st.progress * 100)} aria-label="Download progress">
    <span style:transform="scaleX({st.progress})"></span>
  </div>
  <div class="pct">{Math.round(st.progress * 100)}%</div>
{/if}

{#if error}<p class="error" role="alert">{error}</p>{/if}

{#if !done}
  <button class="install" disabled={busy} onclick={install}>{busy ? 'Installing…' : st.stt === 'error' || st.tts === 'error' ? 'Try again' : 'Install voice models (615 MB download)'}</button>
{:else}
  <p class="ok">Both models are installed. Hold Alt to talk.</p>
{/if}

<style>
  .intro { max-width: 460px; font-size: 14px; line-height: 1.5; color: var(--graphite); }
  .list { margin-top: 22px; border: 1px solid var(--rule); border-radius: 12px; }
  .model { display: flex; align-items: center; justify-content: space-between; gap: 16px; padding: 14px 16px; border-top: 1px solid var(--rule); }
  .model:first-child { border-top: 0; }
  .name { font-size: 14px; font-weight: 500; }
  .what { margin-top: 1px; font-size: 13px; color: var(--faint); }
  .state { font-size: 13px; color: var(--faint); }
  .state.ready { color: var(--text-green); }
  .state.err { color: var(--text-red); }
  .bar { height: 2px; margin-top: 18px; border-radius: 1px; background: var(--rule); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--ink); transform-origin: left; transition: transform 120ms linear; }
  .pct { margin-top: 6px; font-size: 12.5px; color: var(--faint); font-variant-numeric: tabular-nums; }
  .error { margin-top: 14px; font-size: 13.5px; color: var(--text-red); }
  .install { margin-top: 20px; height: 36px; padding: 0 18px; border-radius: 999px; background: var(--ink); color: var(--paper); font-size: 13.5px; transition: opacity 150ms var(--ease-ui), transform 160ms var(--ease-step); }
  .install:disabled { opacity: 0.4; cursor: default; }
  .ok { margin-top: 18px; font-size: 13.5px; color: var(--graphite); }
  /* Gives a little under the press. */
  .install:active:not(:disabled) { transform: scale(0.97); }
  @media (prefers-reduced-motion: reduce) { .install:active:not(:disabled) { transform: none; } }
</style>
