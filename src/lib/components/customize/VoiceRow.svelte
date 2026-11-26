<script lang="ts">
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import type { ComponentProps } from 'svelte';
  import { closeOnEscape } from '../../escape';
  import Reach from './Reach.svelte';
  import VoicePicker from './VoicePicker.svelte';

  type Picker = Omit<ComponentProps<typeof VoicePicker>, 'onclose'>;
  let { desc, value, by, ...picker }: Picker & { desc: string; value: string; by: string } = $props();

  let open = $state(false);
  let root = $state<HTMLElement>();
  let button = $state<HTMLElement>();

  const close = () => (open = false);

  $effect(() => {
    if (!open) return;
    const away = (e: Event) => {
      if (!root?.contains(e.target as Node)) close();
    };
    window.addEventListener('pointerdown', away, true);
    window.addEventListener('focusin', away);
    const off = closeOnEscape(close, () => button);
    return () => {
      window.removeEventListener('pointerdown', away, true);
      window.removeEventListener('focusin', away);
      off();
    };
  });
</script>

<div class="job" bind:this={root}>
  <button class="row" class:open bind:this={button} aria-haspopup="dialog" aria-expanded={open} onclick={() => (open = !open)}>
    <span class="text"><span class="name">{picker.label}</span><span class="desc">{desc}</span></span>
    <span class="val">
      <span class="model" title={value}>{value}</span>
      <span class="by" title={by}>{by}</span>
    </span>
    <span class="chev"><ChevronDown size={14} strokeWidth={1.7} /></span>
  </button>
  {#if open}
    <VoicePicker {...picker} onclose={close} />
    <Reach />
  {/if}
</div>

<style>
  .job { position: relative; }
  .job:not(:first-child) { border-top: 1px solid var(--rule); }
  .row {
    display: flex;
    align-items: baseline;
    gap: 24px;
    width: 100%;
    min-height: 66px;
    padding: 12px 14px 12px 16px;
    text-align: left;
    transition: background-color 150ms var(--ease-ui);
  }
  .row.open { background: var(--shelf); }
  @media (hover: hover) and (pointer: fine) { .row:hover { background: var(--shelf); } }
  .row:focus-visible { outline-offset: -4px; border-radius: 11px; }
  .job:first-child .row { border-top-left-radius: 11px; border-top-right-radius: 11px; }
  .job:last-child .row { border-bottom-left-radius: 11px; border-bottom-right-radius: 11px; }
  .text { flex: 1 1 0; min-width: 176px; }
  .name, .desc, .model, .by { display: block; }
  .name { font-size: 13.5px; line-height: 1.3; font-weight: 500; }
  .desc { margin-top: 3px; font-size: 13px; line-height: 1.4; color: var(--faint); }
  .val { flex: 0 1 auto; min-width: 0; max-width: 250px; text-align: right; }
  .model, .by { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .model { font-size: 13.5px; line-height: 1.3; }
  .by { margin-top: 3px; font-size: 13px; line-height: 1.4; color: var(--graphite); }
  .chev { align-self: center; flex: none; margin-left: -14px; color: var(--faint); }
  .chev :global(svg) { display: block; }
</style>
