<script lang="ts">
  import type { Editor } from '@tiptap/core';
  import FileText from 'lucide-svelte/icons/file-text';
  import { closeOnEscape } from '../escape';
  import { rise } from '../motion/transitions';
  import {
    dismissNoteLink, insertNoteLink, noteLinkMatches, noteLinkMenu,
  } from './notelink.svelte';

  let { editor }: { editor: Editor } = $props();
  const items = $derived(noteLinkMatches(noteLinkMenu.query));

  $effect(() => {
    if (noteLinkMenu.open) return closeOnEscape(dismissNoteLink);
  });

  $effect(() => {
    if (noteLinkMenu.index >= items.length && items.length > 0) {
      noteLinkMenu.index = 0;
    }
  });

  /** The page stage slides with a transform when chat or the sidebar opens, which would re-anchor a fixed menu. */
  function portal(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }
</script>

{#if noteLinkMenu.open && items.length > 0}
  <div
    class="menu"
    role="listbox"
    aria-label="Link to note"
    use:portal
    in:rise={{ y: 4, duration: 140 }}
    style:left="{noteLinkMenu.pos.x}px"
    style:top="{noteLinkMenu.pos.y}px"
  >
    {#each items as item, i (item.id)}
      <div
        class="item"
        class:active={i === noteLinkMenu.index}
        role="option"
        aria-selected={i === noteLinkMenu.index}
        tabindex="-1"
        onmousedown={(e) => {
          e.preventDefault();
          insertNoteLink(editor, item);
        }}
        onmousemove={() => (noteLinkMenu.index = i)}
      >
        <FileText size={15} strokeWidth={1.5} />
        <span class="label">{item.title || 'Untitled'}</span>
      </div>
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 40;
    width: 240px;
    max-height: 264px;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--rule) transparent;
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
    line-height: 1.35;
    color: var(--ink);
    cursor: default;
  }
  .item :global(svg) { color: var(--faint); flex: none; transition: color var(--dur-ui) var(--ease-ui); }
  .item.active { background: var(--riddle-wash); }
  .item.active :global(svg) { color: var(--ink); }
  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
