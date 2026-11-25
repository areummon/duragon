<script lang="ts">
  import { blotPath } from '../ui/blot';

  interface Props {
    size?: number;
    seed?: string;
    /** Lands with a splat and a sheen, and dries. Only for freshly arrived ink. */
    landing?: boolean;
    /** Pulsing while Riddled is thinking. */
    waiting?: boolean;
  }
  let { size = 14, seed = 'duragon', landing = false, waiting = false }: Props = $props();
  const d = $derived(blotPath(seed));
</script>

<svg class="blot" class:landing class:waiting width={size} height={size} viewBox="-13 -13 26 26" aria-hidden="true">
  <path class="body" fill="currentColor" d={d} />
  {#if landing}
    <path class="sheen" d={d} fill="var(--paper)" transform="translate(-1.6 -1.6) scale(0.45)" />
  {/if}
</svg>

<style>
  .blot { flex: none; color: var(--riddle); overflow: visible; }
  .waiting { animation: ink-wait var(--dur-ink-wait) ease-in-out infinite; }
  .landing { animation: blot-splat var(--dur-blot-splat) var(--ease-splat) both; }
  .body, .sheen { transform-box: fill-box; transform-origin: center; }
  .landing .body { animation: ink-dry var(--dur-ink-dry) ease-in forwards; }
  .sheen { opacity: 0; }
  .landing .sheen { animation: ink-sheen var(--dur-ink-sheen) ease-out var(--delay-ink-sheen) both; }
</style>
