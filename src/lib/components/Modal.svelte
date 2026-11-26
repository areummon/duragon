<script lang="ts">
  import X from 'lucide-svelte/icons/x';
  import type { Snippet } from 'svelte';
  import { closeOnEscape } from '../escape';
  import { takeFocus } from '../focus';
  import { pop, veil } from '../motion/transitions';

  interface Props {
    open: boolean;
    onclose: () => void;
    label?: string;
    labelledBy?: string;
    card?: HTMLElement;
    class?: string;
    scrimClass?: string;
    children: Snippet;
  }
  let {
    open,
    onclose,
    label,
    labelledBy,
    card = $bindable(),
    class: className = '',
    scrimClass = '',
    children,
  }: Props = $props();

  $effect(() => {
    if (open) return closeOnEscape(onclose);
  });
</script>

{#if open}
  <div
    class="scrim {scrimClass}"
    role="presentation"
    onpointerdown={(e) => e.target === e.currentTarget && onclose()}
    transition:veil|global
  >
    <div
      class="card {className}"
      bind:this={card}
      role="dialog"
      aria-modal="true"
      aria-label={label}
      aria-labelledby={labelledBy}
      tabindex="-1"
      use:takeFocus
      in:pop|global
      out:pop={{ duration: 140 }}
    >
      <button class="x" aria-label="Close" onclick={onclose}><X size={17} strokeWidth={1.5} /></button>
      {@render children()}
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    place-items: center;
    padding: 24px;
    background: var(--veil);
  }
  .card {
    position: relative;
    border-radius: 14px;
    background: var(--paper);
    box-shadow: var(--lift);
    font-size: 14px;
    line-height: 1.4;
    outline: none;
  }
  .x {
    position: absolute;
    top: 18px;
    right: 18px;
    z-index: 2;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    color: var(--faint);
    transition: color 150ms var(--ease-ui), background-color 150ms var(--ease-ui);
  }
  @media (hover: hover) and (pointer: fine) {
    .x:hover {
      color: var(--ink);
      background: var(--shelf);
    }
  }
</style>
