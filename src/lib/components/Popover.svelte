<script lang="ts">
  import type { Snippet } from 'svelte';
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

  function dismiss(node: HTMLElement) {
    const away = (e: PointerEvent) => {
      const host = node.parentElement;
      if (!host?.contains(e.target as Node)) onclose();
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        e.preventDefault();
        onclose();
      } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
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
    transition:rise={{ y: up ? 2 : -2, duration: 200 }}
  >
    {@render children()}
  </div>
{/if}

<style>
  .popover { top: calc(100% + 6px); left: 0; }
  .popover.right { left: auto; right: 0; }
  .popover.up { top: auto; bottom: calc(100% + 6px); }
</style>
