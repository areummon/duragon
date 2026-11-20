<script lang="ts">
  import type { Editor } from '@tiptap/core';
  import { untrack } from 'svelte';
  import { GUTTER_FROM, gutterPath, measureLines, strokeTiming, underlinePath, type PencilLine } from '../editor/pencil';
  import { findQuote } from '../editor/quote';
  import { ASKING, threads } from '../state/threads.svelte';
  import { blotSvg } from '../ui/blot';

  interface Props { editor: Editor; page: HTMLElement }
  let { editor, page }: Props = $props();

  interface Reading { id: number; target: string; lines: PencilLine[]; phase: 'reading' | 'fading' }
  interface Quiet { id: number; x: number; y: number }

  const FADE_MS = 500;
  const QUIET_MS = 2400;

  let readings = $state<Reading[]>([]);
  let quiets = $state<Quiet[]>([]);
  let seq = 0;

  function linesOf(target: string): PencilLine[] {
    if (editor.view.isDestroyed) return [];
    const r = findQuote(editor.state.doc, target);
    return r ? measureLines(editor.view, r, page) : [];
  }

  function fade(r: Reading) {
    r.phase = 'fading';
    window.setTimeout(() => (readings = readings.filter((x) => x.id !== r.id)), FADE_MS);
  }

  $effect(() => {
    const live = new Set(threads.thinking.filter((t) => ASKING.includes(t.task)).map((t) => t.target));
    untrack(() => {
      for (const r of readings) if (r.phase === 'reading' && !live.has(r.target)) fade(r);
      for (const target of live) {
        if (readings.some((r) => r.target === target && r.phase === 'reading')) continue;
        const lines = linesOf(target);
        if (lines.length) readings = [...readings.filter((r) => r.target !== target), { id: ++seq, target, lines, phase: 'reading' }];
      }
    });
  });

  $effect(() => {
    if (!threads.quiet.length) return;
    untrack(() => {
      for (const q of threads.quiet.splice(0)) {
        const last = linesOf(q.target).at(-1);
        if (!last) continue;
        quiets.push({ id: q.id, x: last.x2 + 10, y: last.y - 19 });
        window.setTimeout(() => (quiets = quiets.filter((x) => x.id !== q.id)), QUIET_MS);
      }
    });
  });

  $effect(() => {
    const view = editor.view;
    let frame = 0;
    const remeasure = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        if (!readings.length) return;
        const next: Reading[] = [];
        for (const r of readings) {
          const lines = linesOf(r.target);
          if (!lines.length) continue;
          if (JSON.stringify(lines) !== JSON.stringify(r.lines)) r.lines = lines;
          next.push(r);
        }
        if (next.length !== readings.length) readings = next;
      });
    };
    const ro = new ResizeObserver(remeasure);
    ro.observe(view.dom);
    ro.observe(page);
    editor.on('update', remeasure);
    return () => {
      cancelAnimationFrame(frame);
      ro.disconnect();
      editor.off('update', remeasure);
    };
  });
</script>

{#each readings as r (r.id)}
  <svg class="pencil" class:fading={r.phase === 'fading'} aria-hidden="true">
    {#if r.lines.length >= GUTTER_FROM}
      {@const d = gutterPath(r.lines)}
      <g style:--draw="0.9s" style:--delay="0s">
        <path class="stroke" {d} pathLength="1" stroke-width="1.4" />
        <path class="stroke grain" {d} pathLength="1" stroke-width="0.7" transform="translate(0.7 0.4)" />
      </g>
    {:else}
      {@const timing = strokeTiming(r.lines)}
      {#each r.lines as l, i (i)}
        {@const d = underlinePath(l, i + 1)}
        <g style:--draw="{timing[i].draw}s" style:--delay="{timing[i].delay}s">
          <path class="stroke" {d} pathLength="1" stroke-width="1.3" />
          <path class="stroke grain" {d} pathLength="1" stroke-width="0.7" transform="translate(0.4 0.7)" />
        </g>
      {/each}
    {/if}
  </svg>
  {#if r.phase === 'reading'}
    {@const last = r.lines[r.lines.length - 1]}
    <span class="reading-mark" style:left="{last.x2 + 10}px" style:top="{last.mid}px">{@html blotSvg('reading', 11)}</span>
  {/if}
{/each}
{#each quiets as q (q.id)}
  <span class="quiet-mark" style:left="{q.x}px" style:top="{q.y}px">Nothing to add.</span>
{/each}

<style>
  .pencil {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
    z-index: 4;
    transition: opacity 500ms ease-out;
  }
  .pencil.fading { opacity: 0; }
  .stroke {
    fill: none;
    stroke: var(--riddle);
    stroke-opacity: 0.55;
    stroke-linecap: round;
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    animation: pencil-draw var(--draw) linear var(--delay) forwards;
  }
  .grain { stroke-opacity: 0.3; }

  .reading-mark {
    position: absolute;
    display: flex;
    translate: 0 -50%;
    color: var(--riddle);
    pointer-events: none;
    z-index: 4;
  }
  .reading-mark :global(.blot) { animation: reading-breathe var(--dur-breathe) ease-in-out infinite; }

  .quiet-mark {
    position: absolute;
    font-family: var(--font-voice);
    font-size: 14px;
    line-height: 1;
    white-space: nowrap;
    color: var(--graphite);
    pointer-events: none;
    z-index: 4;
    opacity: 0;
    animation: quiet-mark var(--dur-quiet-mark) ease-out forwards;
  }

  @media (prefers-reduced-motion: reduce) {
    .stroke { stroke-dashoffset: 0; animation: none; }
    .reading-mark :global(.blot) { animation: none; opacity: 0.6; }
    .quiet-mark { animation: none; opacity: 0.8; }
  }
</style>
