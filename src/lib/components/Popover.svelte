<script lang="ts">
  import type { Snippet } from 'svelte';
  import { closeOnEscape } from '../escape';
  import { rise } from '../motion/transitions';

  interface Props {
    open: boolean;
    onclose: () => void;
    align?: 'left' | 'right';
    up?: boolean;
    at?: { x: number; y: number };
    width?: number;
    label?: string;
    children: Snippet;
  }
  let { open, onclose, align = 'left', up = false, at, width, label, children }: Props = $props();

  $effect(() => {
    if (open) return closeOnEscape(() => onclose());
  });

  function dismiss(node: HTMLElement) {
    // A menu under its button counts the button as inside, so pressing it toggles. One opened at the pointer has no
    // button beside it, and its parent can hold the whole page, so only the menu itself is inside.
    const away = (e: PointerEvent) => {
      const inside = at ? node : node.parentElement;
      if (!inside?.contains(e.target as Node)) onclose();
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        const items = [...node.querySelectorAll<HTMLElement>('[role^="menuitem"]')];
        const i = items.indexOf(document.activeElement as HTMLElement);
        const next = e.key === 'ArrowDown' ? i + 1 : i - 1;
        items[(next + items.length) % items.length]?.focus();
        e.preventDefault();
      }
    };
    window.addEventListener('pointerdown', away, true);
    window.addEventListener('keydown', key, true);
    return {
      destroy() {
        window.removeEventListener('pointerdown', away, true);
        window.removeEventListener('keydown', key, true);
      },
    };
  }
</script>

{#if open}
  <div
    class="popover"
    class:up
    class:right={align === 'right'}
    role="menu"
    aria-label={label}
    style:position={at ? 'fixed' : undefined}
    style:left={at ? `${at.x}px` : undefined}
    style:top={at ? `${at.y}px` : undefined}
    style:min-width={width ? `${width}px` : undefined}
    use:dismiss
    transition:rise={{ y: up ? 4 : -4, scale: 0.97, duration: 180 }}
  >
    {@render children()}
  </div>
{/if}

<style>
  /* Grows out of the button it opened from; a menu at the pointer grows from its top left corner. */
  .popover { top: calc(100% + 6px); left: 0; transform-origin: top left; }
  .popover.right { left: auto; right: 0; transform-origin: top right; }
  .popover.up { top: auto; bottom: calc(100% + 6px); transform-origin: bottom left; }
</style>
