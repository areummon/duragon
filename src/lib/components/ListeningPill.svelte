<script lang="ts">
  import { rise } from '../motion/transitions';
  import { voice } from '../state/voice.svelte';

  const idle = $derived(Math.max(...voice.levels) < 0.02);
</script>

<div class="pill" role="status" aria-live="polite" transition:rise={{ y: 4, duration: 160 }}>
  <span class="bars" class:idle aria-hidden="true">
    {#each voice.levels as level, i (i)}
      <span class="bar" style:--i={i} style:transform={idle ? undefined : `scaleY(${0.2 + 0.8 * Math.min(1, level)})`}></span>
    {/each}
  </span>
  <span>{voice.mode === 'hands' ? 'Listening, hands-free' : 'Listening'}</span>
</div>

<style>
  .pill {
    position: fixed;
    right: 24px;
    bottom: 20px;
    z-index: 25;
    display: flex;
    align-items: center;
    gap: 9px;
    height: 30px;
    padding: 0 14px 0 12px;
    border: 1px solid var(--rule);
    border-radius: 999px;
    background: var(--paper);
    font-size: 12.5px;
    line-height: 1;
    color: var(--graphite);
    pointer-events: none;
  }
  .bars { display: flex; align-items: center; gap: 2px; height: 14px; }
  .bar {
    width: 2px;
    height: 14px;
    border-radius: 1px;
    background: var(--riddle);
    transform: scaleY(0.25);
    transition: transform 90ms linear;
  }
  .idle .bar { animation: voice-bar 900ms ease-in-out infinite; animation-delay: calc(var(--i) * 110ms); }
</style>
