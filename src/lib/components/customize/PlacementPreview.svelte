<script lang="ts">
  let { kind }: { kind: 'both' | 'right' | 'marks' } = $props();
</script>

<svg viewBox="0 0 160 96" aria-hidden="true">
  <rect width="160" height="96" rx="7" class="bg" />
  {#if kind === 'both'}
    <path d="M62 18h26M62 31h38M62 40h34M62 56h30M62 65h36M62 74h22" class="line" />
    <path d="M22 54h26M26 62h18" class="ink" />
    <path d="M100 28h28M100 36h20" class="ink" />
    <path d="M50 56c4 0 6-1 10-4M101 31c-4 0-6 1-10 4" class="tether" />
  {:else if kind === 'right'}
    <path d="M26 18h26M26 31h38M26 40h34M26 56h30M26 65h36M26 74h22" class="line" />
    <path d="M96 30h34M96 38h26M96 58h34M96 66h22" class="ink" />
    <path d="M66 35c8 0 10-2 22-4M60 61c10 0 12-2 28-3" class="tether" />
  {:else}
    <path d="M40 18h26" class="line" />
    <path d="M40 34h70" class="line" />
    <circle cx="118" cy="34" r="3.2" class="ring" />
    <rect x="36" y="44" width="88" height="24" rx="6" class="card" />
    <path d="M44 54h50M44 61h28" class="ink" />
    <path d="M40 78h58" class="line" />
  {/if}
</svg>

<style>
  svg { display: block; width: 100%; height: auto; }
  .bg { fill: var(--shelf); }
  path { fill: none; stroke-linecap: round; stroke-width: 2.4; }
  .line { stroke: var(--faint); opacity: 0.8; }
  .ink { stroke: var(--ink); }
  .tether { stroke: var(--faint); stroke-width: 1; }
  .ring { fill: none; stroke: var(--ink); stroke-width: 1.2; }
  .card { fill: none; stroke: var(--faint); stroke-width: 1; }
</style>
