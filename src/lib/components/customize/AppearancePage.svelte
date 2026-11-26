<script lang="ts">
  import type { Settings } from '../../api/types';
  import { settings, updateSettings } from '../../state/settings.svelte';
  import Choice from './Choice.svelte';

  const s = $derived(settings.value);
  const set = (patch: Partial<Settings>) => void updateSettings(patch);

  const INKS = [
    ['ink', 'Ink', 'var(--ink)'],
    ['blue', 'Blue', 'var(--text-blue)'],
    ['green', 'Green', 'var(--text-green)'],
    ['red', 'Red', 'var(--text-red)'],
    ['purple', 'Purple', 'var(--text-purple)'],
    ['brown', 'Brown', 'var(--text-brown)'],
    ['orange', 'Orange', 'var(--text-orange)'],
  ] as const;
</script>

<div class="group">
  <h4 id="g-app">Theme</h4>
  <div class="grid three" role="radiogroup" aria-labelledby="g-app">
    {#each [['system', 'System'], ['light', 'Light'], ['dark', 'Dark']] as const as [id, name] (id)}
      <Choice selected={s.appearance === id} onpick={() => set({ appearance: id })} title={name} />
    {/each}
  </div>
</div>

<div class="group">
  <h4 id="g-size">Text size</h4>
  <div class="grid three" role="radiogroup" aria-labelledby="g-size">
    {#each [['small', 'Small', 15], ['default', 'Default', 16], ['large', 'Large', 18]] as const as [id, name, px] (id)}
      <Choice selected={s.textSize === id} onpick={() => set({ textSize: id })} title={name}>
        <span class="sample" style:font-size="{px}px">A thought.</span>
      </Choice>
    {/each}
  </div>
</div>

<div class="group">
  <div class="line">
    <div>
      <h4>Page width</h4>
      <p class="note tight">Drag the two ticks above the page to change it.</p>
    </div>
    <button class="ghost" disabled={s.pageWidth === null} onclick={() => set({ pageWidth: null })}>Reset</button>
  </div>
</div>

<div class="group">
  <h4 id="g-let">Duragon’s lettering</h4>
  <div class="grid two" role="radiogroup" aria-labelledby="g-let">
    <Choice selected={s.lettering === 'own'} onpick={() => set({ lettering: 'own' })} title="Duragon’s own">
      <span class="sample serif">Does this still hold?</span>
    </Choice>
    <Choice selected={s.lettering === 'notes'} onpick={() => set({ lettering: 'notes' })} title="Like your notes">
      <span class="sample sans">Does this still hold?</span>
    </Choice>
  </div>
</div>

<div class="group">
  <h4 id="g-col">Duragon’s colour</h4>
  <div class="swatches" role="radiogroup" aria-labelledby="g-col">
    {#each INKS as [id, name, colour] (id)}
      <button class="swatch" class:selected={s.inkColour === id} role="radio" aria-checked={s.inkColour === id} aria-label={name} title={name} onclick={() => set({ inkColour: id })}>
        <span style:background={colour}></span>
      </button>
    {/each}
  </div>
</div>

<style>
  .group { margin-top: 22px; }
  .group:first-child { margin-top: 0; }
  h4 { margin: 0 0 10px; font-size: 13.5px; line-height: 1.3; font-weight: 500; color: var(--ink); }
  .line h4 { margin-bottom: 0; }
  .grid { display: grid; gap: 10px; }
  .two { grid-template-columns: 1fr 1fr; }
  .three { grid-template-columns: repeat(3, 1fr); }
  .note { margin-top: 8px; font-size: 13px; line-height: 1.4; color: var(--faint); }
  .note.tight { margin-top: 3px; }
  .sample { margin-top: 4px; line-height: 1.3; color: var(--ink); }
  .sample.serif { font-family: 'Newsreader Variable', Georgia, serif; font-size: 16px; }
  .sample.sans { font-family: var(--font-sans); font-size: 15px; }

  .line { display: flex; align-items: center; justify-content: space-between; gap: 20px; }
  .ghost { height: 30px; padding: 0 14px; border: 1px solid var(--rule); border-radius: 999px; font-size: 13px; transition: opacity 150ms var(--ease-ui), transform 160ms var(--ease-step); }
  .ghost:disabled { opacity: 0.4; cursor: default; }

  .swatches { display: flex; gap: 12px; }
  .swatch { display: grid; place-items: center; width: 34px; height: 34px; border-radius: 50%; border: 1.5px solid transparent; transition: border-color 150ms var(--ease-ui); }
  .swatch span { width: 20px; height: 20px; border-radius: 50%; }
  .swatch.selected { border-color: var(--ink); }
  /* Gives a little under the press. */
  .ghost:active:not(:disabled) { transform: scale(0.97); }
  @media (prefers-reduced-motion: reduce) { .ghost:active:not(:disabled) { transform: none; } }
</style>
