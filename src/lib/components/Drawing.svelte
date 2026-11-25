<script lang="ts">
  import { untrack } from 'svelte';
  import ChevronLeft from 'lucide-svelte/icons/chevron-left';
  import ChevronRight from 'lucide-svelte/icons/chevron-right';
  import RotateCcw from 'lucide-svelte/icons/rotate-ccw';
  import type { DrawingState } from '../api/types';
  import Blot from './Blot.svelte';

  interface Props {
    status: DrawingState;
    svg: string | null;
    /** Auto-plays the steps in. Stored drawings show every step at once. */
    fresh?: boolean;
    size?: 'margin' | 'large' | 'dialog';
    onLarger?: () => void;
  }
  let { status, svg, fresh = false, size = 'margin', onLarger }: Props = $props();

  const STEP_MS = 1300;
  const reduced = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
  const DRAWABLE = 'path, line, polyline, polygon, rect, circle, ellipse';
  const SVG_NS = 'http://www.w3.org/2000/svg';
  const PAD = 6;
  const uid = Math.random().toString(36).slice(2, 9);
  /** The arrowheads a drawing may name, by the colour of the line they finish. */
  const HEADS: Record<string, string> = { arrow: 'var(--ink)', 'arrow-quiet': 'var(--graphite)' };

  function head(name: string): SVGMarkerElement {
    const marker = document.createElementNS(SVG_NS, 'marker');
    const attrs = { id: `${name}-${uid}`, viewBox: '0 0 8 8', refX: '7', refY: '4', markerWidth: '7', markerHeight: '7', orient: 'auto-start-reverse' };
    for (const [k, v] of Object.entries(attrs)) marker.setAttribute(k, v);
    const path = document.createElementNS(SVG_NS, 'path');
    path.setAttribute('d', 'M 1 1 L 7 4 L 1 7');
    path.setAttribute('style', `fill:none;stroke:${HEADS[name]};stroke-width:1.2;stroke-linecap:round;stroke-linejoin:round`);
    marker.append(path);
    return marker;
  }

  /** Gives the drawing its arrowheads and trims the empty space above and below what was drawn. */
  function frame(svg: SVGSVGElement) {
    const used = new Set<string>();
    for (const el of svg.querySelectorAll('[marker-end], [marker-start]'))
      for (const attr of ['marker-end', 'marker-start']) {
        const name = el.getAttribute(attr)?.match(/^url\(#([\w-]+)\)$/)?.[1];
        if (!name || !(name in HEADS)) continue;
        used.add(name);
        el.setAttribute(attr, `url(#${name}-${uid})`);
      }
    if (used.size) {
      const defs = document.createElementNS(SVG_NS, 'defs');
      for (const name of used) defs.append(head(name));
      svg.prepend(defs);
    }
    const box = svg.getBBox();
    const view = svg.viewBox.baseVal;
    if (box.height > 0 && view.width > 0) svg.setAttribute('viewBox', `${view.x} ${box.y - PAD} ${view.width} ${box.height + PAD * 2}`);
  }

  let host = $state<HTMLElement>();
  let steps: SVGGElement[] = [];
  let captions = $state<string[]>([]);
  let shown = $state(0);
  let timer: number | undefined;

  const total = $derived(captions.length);
  const caption = $derived(captions[Math.max(shown, 1) - 1] ?? '');

  function reveal(i: number, animated: boolean) {
    const g = steps[i];
    g.getAnimations({ subtree: true }).forEach((a) => a.cancel());
    g.style.opacity = '1';
    if (!animated) return;
    g.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 450, easing: 'cubic-bezier(0.22, 1, 0.36, 1)' });
    if (reduced()) return;
    g.querySelectorAll<SVGGeometryElement>(DRAWABLE).forEach((p) => {
      p.setAttribute('pathLength', '1');
      p.style.strokeDasharray = '1';
      const a = p.animate([{ strokeDashoffset: 1 }, { strokeDashoffset: 0 }], { duration: 600, easing: 'ease-out' });
      a.onfinish = a.oncancel = () => p.style.removeProperty('stroke-dasharray');
    });
  }

  function hide(i: number, animated: boolean) {
    const g = steps[i];
    g.getAnimations({ subtree: true }).forEach((a) => a.cancel());
    g.style.opacity = '0';
    if (animated) g.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 150 });
  }

  function stop() {
    window.clearTimeout(timer);
    timer = undefined;
  }

  function goto(n: number, animated = true) {
    stop();
    const prev = shown;
    shown = Math.max(0, Math.min(total, n));
    steps.forEach((_, i) => {
      if (i < shown && i >= prev) reveal(i, animated);
      else if (i >= shown && i < prev) hide(i, animated);
      else if (i < shown) steps[i].style.opacity = '1';
      else steps[i].style.opacity = '0';
    });
  }

  function play() {
    stop();
    goto(0, false);
    const tick = () => {
      goto(shown + 1);
      if (shown < total) timer = window.setTimeout(tick, STEP_MS);
    };
    timer = window.setTimeout(tick, 120);
  }

  $effect(() => {
    if (!host || status !== 'done' || !svg) return;
    const el = host;
    untrack(() => {
      const svgEl = el.querySelector('svg');
      if (svgEl) frame(svgEl);
      steps =[...el.querySelectorAll<SVGGElement>('g[data-step]')];
      captions = steps.map((g) => g.getAttribute('data-caption') ?? '');
      if (steps.length === 0) return;
      shown = 0;
      if (fresh) play();
      else goto(steps.length, false);
    });
    return stop;
  });
</script>

{#if status === 'pending'}
  <div class="wait"><Blot size={15} seed="sketch" waiting /><span>Sketching it…</span></div>
{:else if status === 'failed'}
  <div class="failed">Couldn’t sketch that.</div>
{:else if status === 'done' && svg}
  <figure class="drawing {size}">
    <div class="canvas" bind:this={host}>{@html svg}</div>
    {#if total > 0}
      <div class="player">
        <span class="caption">{caption}</span>
        <span class="controls">
          <button aria-label="Previous step" disabled={shown <= 1} onclick={() => goto(shown - 1)}><ChevronLeft size={14} strokeWidth={1.6} /></button>
          <span class="count">{Math.max(shown, 1)}/{total}</span>
          <button aria-label="Next step" disabled={shown >= total} onclick={() => goto(shown + 1)}><ChevronRight size={14} strokeWidth={1.6} /></button>
          <button aria-label="Replay" onclick={play}><RotateCcw size={13} strokeWidth={1.6} /></button>
        </span>
      </div>
    {/if}
    {#if onLarger && size !== 'dialog'}
      <button class="larger" onclick={onLarger}>See it larger</button>
    {/if}
  </figure>
{/if}

<style>
  /* Sketches keep the serif lettering whichever lettering the notes use, as in Riddled. */
  .drawing { margin: 0; width: 100%; --font-voice: var(--font-serif); }
  .drawing.margin { max-width: 320px; }
  .drawing.dialog { max-width: 640px; margin-inline: auto; }
  .canvas :global(svg) {
    display: block;
    width: 100%;
    height: auto;
    overflow: visible;
    font-weight: 420;
    font-variation-settings: 'opsz' 14;
    letter-spacing: 0.004em;
  }
  /* While the page width is dragged, each sketch gets its own layer, or WebKitGTK repaints every sketch on each frame the
     toolbar glides and the drag halves its frame rate. Not at rest: a layer at a fractional offset paints a touch softer. */
  :global(.page.dragging) .canvas :global(svg) { will-change: transform; }
  /* A paper-coloured halo behind each label, so lines fade out just before they reach the words. */
  .canvas :global(text) { paint-order: stroke; stroke: var(--paper) !important; stroke-width: 4px !important; stroke-linejoin: round; }
  /* As in Riddled, a sketch fills the column up to 440 px tall; the width cap keeps a short one from growing past 1.3×. */
  .large .canvas :global(svg) { max-height: 440px; max-width: 416px; margin-inline: auto; }
  .player {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-top: 10px;
    min-height: 24px;
  }
  .caption {
    font-family: var(--font-voice);
    font-size: 15px;
    line-height: 1.4;
    color: var(--graphite);
  }
  .controls { font-family: var(--font-sans); display: inline-flex; align-items: center; gap: 2px; flex: none; color: var(--faint); font-size: 12px; line-height: 24px; }
  .controls button {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    transition: opacity var(--dur-ui) var(--ease-ui);
  }
  .controls button:hover:not(:disabled) { color: var(--ink); }
  .controls button:disabled { opacity: 0.35; cursor: default; }
  .count { min-width: 28px; text-align: center; font-variant-numeric: tabular-nums; }
  .larger {
    font-family: var(--font-sans);
    display: block;
    margin-top: 4px;
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--faint);
    transition: color var(--dur-ui) var(--ease-ui);
  }
  .larger:hover { color: var(--graphite); }
  .wait {
    display: flex;
    align-items: center;
    gap: 9px;
    font-family: var(--font-voice);
    font-size: 15px;
    color: var(--faint);
    padding: 6px 0;
  }
  .failed { font-family: var(--font-voice); font-size: 14px; color: var(--faint); padding: 6px 0; }
</style>
