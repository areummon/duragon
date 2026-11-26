<script lang="ts">
  import type { Snippet } from 'svelte';
  import { dismissOnOutside } from '../escape';
  import { menuKeys } from '../focus';
  import { rise } from '../motion/transitions';

  interface Props {
    open: boolean;
    onclose: () => void;
    align?: 'left' | 'right';
    up?: boolean;
    at?: { x: number; y: number };
    width?: number;
    label?: string;
    role?: 'menu' | 'listbox';
    children: Snippet;
  }
  let { open, onclose, align = 'left', up = false, at, width, label, role = 'menu', children }: Props = $props();

  let el = $state<HTMLElement>();

  $effect(() => {
    if (!open || !el) return;
    const node = el;
    // A menu under its button counts the button as inside, so pressing it toggles. One opened at the pointer has no
    // button beside it, and its parent can hold the whole page, so only the menu itself is inside.
    return dismissOnOutside(() => (at ? node : node.parentElement), () => onclose());
  });
</script>

{#if open}
  <div
    class="popover"
    class:up
    class:right={align === 'right'}
    bind:this={el}
    {role}
    aria-label={label}
    style:position={at ? 'fixed' : undefined}
    style:left={at ? `${at.x}px` : undefined}
    style:top={at ? `${at.y}px` : undefined}
    style:min-width={width ? `${width}px` : undefined}
    use:menuKeys={() => onclose()}
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
