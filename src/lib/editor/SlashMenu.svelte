<script lang="ts">
  import type { Editor } from '@tiptap/core';
  import { closeOnEscape } from '../escape';
  import { dismissSlash, runSlash, slash, slashMatches } from './slash.svelte';

  let { editor }: { editor: Editor } = $props();
  const items = $derived(slashMatches(slash.query));
  $effect(() => {
    if (slash.open) return closeOnEscape(dismissSlash);
  });

  /** The page stage slides with a transform when chat or the sidebar opens, which would re-anchor a fixed menu. */
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }
</script>

{#if slash.open}
  <div
    class="menu"
    role="listbox"
    aria-label="Commands"
    use:portal
    style:left="{slash.x}px"
    style:top="{slash.y}px"
  >
    {#each items as item, i (item.id)}
      {@const Icon = item.icon}
      <div
        class="item"
        class:active={i === slash.index}
        role="option"
        aria-selected={i === slash.index}
        tabindex="-1"
        onmousedown={(e) => {
          e.preventDefault();
          runSlash(editor, item);
        }}
        onmousemove={() => (slash.index = i)}
      >
        <Icon size={15} strokeWidth={1.5} />
        <span>{item.label}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 40;
    width: 220px;
    padding: 4px;
    background: var(--paper);
    border: 1px solid var(--rule);
    border-radius: 12px;
    box-shadow: var(--lift);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 33px;
    padding: 0 10px;
    border-radius: 8px;
    font-size: 14px;
    line-height: 1;
    color: var(--ink);
    cursor: default;
  }
  .item :global(svg) { color: var(--faint); flex: none; }
  .item.active { background: var(--riddle-wash); }
</style>
