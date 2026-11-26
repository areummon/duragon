<script lang="ts" generics="T">
  import type { Editor } from '@tiptap/core';
  import type { Snippet } from 'svelte';
  import { closeOnEscape } from '../escape';
  import { rise } from '../motion/transitions';
  import { portal } from '../portal';
  import type { InlineMenu } from './inline-menu.svelte';

  interface Props {
    editor: Editor;
    menu: InlineMenu<T>;
    label: string;
    itemKey: (item: T) => string;
    animate?: boolean;
    children: Snippet<[T]>;
  }

  let { editor, menu, label, itemKey, animate = false, children }: Props = $props();
  const items = $derived(menu.matches(menu.state.query));

  $effect(() => {
    if (menu.state.open) return closeOnEscape(menu.dismiss);
  });

  $effect(() => {
    if (menu.state.index >= items.length && items.length > 0) {
      menu.state.index = 0;
    }
  });
</script>

{#if menu.state.open && items.length > 0}
  <div
    class="menu"
    role="listbox"
    aria-label={label}
    use:portal
    in:rise={animate ? { y: 4, duration: 140 } : { y: 0, duration: 0 }}
    style:width="{menu.width}px"
    style:left="{menu.state.x}px"
    style:top="{menu.state.y}px"
  >
    {#each items as item, i (itemKey(item))}
      <div
        class="item"
        class:active={i === menu.state.index}
        role="option"
        aria-selected={i === menu.state.index}
        tabindex="-1"
        onmousedown={(e) => {
          e.preventDefault();
          menu.choose(editor, item);
        }}
        onmousemove={() => (menu.state.index = i)}
      >
        {@render children(item)}
      </div>
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 40;
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
  .item :global(svg) {
    color: var(--faint);
    flex: none;
    transition: color var(--dur-ui) var(--ease-ui);
  }
  .item.active {
    background: var(--riddle-wash);
  }
  .item.active :global(svg) {
    color: var(--ink);
  }
</style>
