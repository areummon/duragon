<script lang="ts">
  import { onMount } from 'svelte';
  import { unfold } from '../../motion/transitions';

  /**
   * Goes right after an open popover. Scrolls it into view, holds the page's height while the popover shrinks, and
   * gives the height back as the popover leaves, so the page glides instead of jumping.
   */
  const GAP = 12;
  let node: HTMLElement;

  onMount(() => {
    const popover = node.previousElementSibling as HTMLElement;
    const scroller = node.closest<HTMLElement>('.scroll');
    const still = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    let height = 0;
    const grown = new ResizeObserver(() => {
      // Layout sizes, since the popover is still scaling up.
      const needs = popover.offsetHeight + GAP;
      if (needs <= height || !scroller) return;
      height = needs;
      node.style.top = `${popover.offsetTop}px`;
      node.style.height = `${height}px`;
      // A frame later: WebKit holds a smooth scroll to the height the page had before.
      requestAnimationFrame(() => {
        const below = node.getBoundingClientRect().bottom - scroller.getBoundingClientRect().bottom;
        if (below > 0) scroller.scrollBy({ top: below, behavior: still ? 'auto' : 'smooth' });
      });
    });
    grown.observe(popover);
    return () => grown.disconnect();
  });
</script>

<div class="reach" bind:this={node} out:unfold={{ duration: 200 }}></div>

<style>
  .reach { position: absolute; left: 0; width: 1px; pointer-events: none; }
</style>
