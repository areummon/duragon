<script lang="ts">
  import X from 'lucide-svelte/icons/x';
  import { lightbox } from '../state/layout.svelte';
  import { closeOnEscape } from '../escape';
  import { takeFocus } from '../focus';
  import { pop, veil } from '../motion/transitions';
  import Drawing from './Drawing.svelte';

  const close = () => (lightbox.svg = null);
  $effect(() => {
    if (lightbox.svg) return closeOnEscape(close);
  });
</script>

{#if lightbox.svg}
  <div class="scrim" role="presentation" onclick={(e) => e.target === e.currentTarget && close()} transition:veil|global>
    <div class="card" role="dialog" aria-modal="true" aria-label="The drawing, larger" tabindex="-1" use:takeFocus in:pop|global out:pop={{ duration: 140 }}>
      <header>
        <h2>The drawing, larger</h2>
        <button aria-label="Close" onclick={close}><X size={17} strokeWidth={1.5} /></button>
      </header>
      <Drawing status="done" svg={lightbox.svg} fresh size="dialog" />
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    padding: 24px;
    background: color-mix(in srgb, var(--paper) 55%, transparent);
  }
  .card {
    width: min(760px, 100%);
    max-height: calc(100vh - 48px);
    overflow: auto;
    padding: 22px 28px 30px;
    border-radius: 14px;
    background: var(--paper);
    box-shadow: var(--lift);
    outline: none;
  }
  header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 18px; }
  h2 { margin: 0; font-family: var(--font-voice); font-size: 18px; font-weight: 400; color: var(--riddle); }
  header button {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 1px solid var(--rule);
    border-radius: 8px;
    background: var(--shelf);
    color: var(--graphite);
    transition:
      color 150ms var(--ease-ui),
      background-color 150ms var(--ease-ui),
      border-color 150ms var(--ease-ui),
      transform 140ms var(--ease-step);
  }
  @media (hover: hover) and (pointer: fine) {
    header button:hover {
      color: var(--ink);
      background: var(--riddle-wash);
      border-color: color-mix(in srgb, var(--ink) 20%, var(--rule));
    }
  }
  header button:active { transform: scale(0.95); }
  @media (prefers-reduced-motion: reduce) { header button:active { transform: none; } }
</style>
