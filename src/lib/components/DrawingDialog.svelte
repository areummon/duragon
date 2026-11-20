<script lang="ts">
  import X from 'lucide-svelte/icons/x';
  import { lightbox } from '../state/layout.svelte';
  import { pop, veil } from '../motion/transitions';
  import Drawing from './Drawing.svelte';

  const close = () => (lightbox.svg = null);
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && lightbox.svg && (e.stopPropagation(), close())} />

{#if lightbox.svg}
  <div class="scrim" role="presentation" onclick={(e) => e.target === e.currentTarget && close()} transition:veil>
    <div class="card" role="dialog" aria-modal="true" aria-label="The drawing, larger" tabindex="-1" in:pop out:pop={{ duration: 140 }}>
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
  }
  header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 18px; }
  h2 { margin: 0; font-family: var(--font-voice); font-size: 18px; font-weight: 400; color: var(--riddle); }
  header button { display: grid; place-items: center; width: 28px; height: 28px; margin-right: -6px; color: var(--faint); transition: color var(--dur-ui) var(--ease-ui); }
  header button:hover { color: var(--ink); }
</style>
