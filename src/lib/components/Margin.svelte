<script lang="ts">
  import type { Thread as ThreadModel } from '../api/types';
  import { MARGIN_GAP, marginWidth, useMarks } from '../state/layout.svelte';
  import { settings } from '../state/settings.svelte';
  import { voice } from '../state/voice.svelte';
  import { geometry, threads, visibleThreads } from '../state/threads.svelte';
  import Thread from './Thread.svelte';

  const GAP = 16;
  const BLOT_C = -0.5;
  const BLOT_Y = 12.5;

  let heights = $state<Record<string, number>>({});

  interface Placed {
    thread: ThreadModel;
    side: 'left' | 'right';
    y: number;
    orphan: boolean;
  }

  const width = $derived(marginWidth());

  const placed = $derived.by<Placed[]>(() => {
    if (settings.value.justMyWords) return [];
    const marks = useMarks();
    const all = visibleThreads();
    if (marks) return [];
    const shown = all.filter((t) => t.kind === 'question' || t.id === threads.openId);
    const order = [...shown].filter((t) => t.kind === 'question').sort((a, b) => a.createdAt - b.createdAt);
    const sideOf = (t: ThreadModel): 'left' | 'right' =>
      settings.value.questionPlacement === 'both' && !marks && t.kind === 'question' && order.indexOf(t) % 2 === 1 ? 'left' : 'right';

    const anchored: { t: ThreadModel; top: number; orphan: boolean }[] = [];
    for (const t of shown) {
      const g = geometry.byId[t.id];
      if (g === undefined) continue;
      anchored.push({ t, top: g ? g.top : Number.POSITIVE_INFINITY, orphan: g === null });
    }
    anchored.sort((a, b) => a.top - b.top || a.t.createdAt - b.t.createdAt);

    const bottoms = { left: -GAP, right: -GAP };
    const out: Placed[] = [];
    for (const a of anchored) {
      const side = sideOf(a.t);
      const y = a.orphan ? bottoms[side] + GAP + 24 : Math.max(a.top, bottoms[side] + GAP);
      bottoms[side] = y + (heights[a.t.id] ?? 60);
      out.push({ thread: a.t, side, y, orphan: a.orphan });
    }
    return out;
  });

  const target = $derived.by(() => {
    const id = threads.openId ?? threads.hoverId;
    const p = placed.find((x) => x.thread.id === id && !x.orphan);
    const g = id ? geometry.byId[id] : null;
    return p && g ? { p, g, open: p.thread.id === threads.openId } : null;
  });

  function tether(p: Placed, g: { endX: number; endY: number }) {
    const sx = g.endX + 3;
    const sy = g.endY;
    const bx = p.side === 'right' ? geometry.pageWidth + MARGIN_GAP + BLOT_C - 11 : -MARGIN_GAP - BLOT_C + 11;
    const by = p.y + BLOT_Y;
    const mx = (sx + bx) / 2;
    return `M${sx} ${sy}C${mx} ${sy} ${mx} ${by} ${bx} ${by}`;
  }

  const focusId = $derived(
    threads.openId ?? (voice.playing ? (threads.list.find((t) => t.messages.some((m) => m.id === voice.speakingId))?.id ?? null) : null),
  );
</script>

{#if placed.length}
  <div class="margins" style:--mw="{width}px" style:--gap="{MARGIN_GAP}px">
    {#if target}
      {#key `${target.p.thread.id}:${target.open}`}
        <svg class="tether" aria-hidden="true">
          <path d={tether(target.p, target.g)} pathLength="1" />
        </svg>
      {/key}
    {/if}
    {#each placed as p (p.thread.id)}
      <div
        class="slot {p.side}"
        class:orphan={p.orphan}
        class:dim={focusId !== null && focusId !== p.thread.id}
        style:transform="translateY({p.y}px)"
        bind:clientHeight={heights[p.thread.id]}
      >
        <Thread thread={p.thread} side={p.side} />
      </div>
    {/each}
  </div>
{/if}

<style>
  .margins { position: absolute; inset: 0; pointer-events: none; z-index: 5; }
  .slot {
    position: absolute;
    top: 0;
    width: var(--mw);
    pointer-events: auto;
    transition: transform 340ms var(--ease-step), opacity 340ms var(--ease-step);
  }
  .slot.right { left: calc(100% + var(--gap)); }
  .slot.left { right: calc(100% + var(--gap)); }
  .slot.orphan { opacity: 0.5; }
  .slot.dim { opacity: 0.4; }
  .tether { position: absolute; inset: 0; width: 100%; height: 100%; overflow: visible; }
  .tether path {
    fill: none;
    stroke: var(--riddle-line);
    stroke-width: 1;
    stroke-linecap: round;
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    animation: tether-draw var(--dur-tether) var(--ease-tether) var(--delay-tether) both;
  }
</style>
