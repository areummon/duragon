<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    selected: boolean;
    onpick: () => void;
    title: string;
    desc?: string;
    preview?: Snippet;
    children?: Snippet;
    label?: string;
  }
  let { selected, onpick, title, desc, preview, children, label }: Props = $props();
</script>

<button class="choice" class:selected role="radio" aria-checked={selected} aria-label={label} onclick={onpick}>
  {#if preview}<span class="preview">{@render preview()}</span>{/if}
  <span class="title">{title}</span>
  {#if desc}<span class="desc">{desc}</span>{/if}
  {@render children?.()}
</button>

<style>
  .choice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 12px;
    border: 1px solid var(--rule);
    border-radius: 12px;
    text-align: left;
    color: var(--ink);
    transition: border-color 150ms var(--ease-ui), background-color 150ms var(--ease-ui);
  }
  .choice.selected { border-color: var(--ink); }
  @media (hover: hover) and (pointer: fine) {
    .choice:not(.selected):hover { border-color: var(--faint); }
  }
  .title { font-size: 13.5px; line-height: 1.3; font-weight: 500; }
  .desc { font-size: 13px; line-height: 1.4; color: var(--faint); }
  .preview { display: block; width: 100%; margin-bottom: 8px; }
</style>
