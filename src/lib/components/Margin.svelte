<script lang="ts">
  import type { Thread as ThreadModel } from '../api/types';
  import { MARGIN_GAP, layout, marginWidth, useMarks } from '../state/layout.svelte';
  import { settings } from '../state/settings.svelte';
  import { voice } from '../state/voice.svelte';
  import { geometry, threads, visibleThreads } from '../state/threads.svelte';
  import Tether from './Tether.svelte';
  import Thread from './Thread.svelte';

  const GAP = 16;
  const BLOT_C = -0.5;
  const BLOT_Y = 12.5;
  const QUOTE_GAP = 9;

  let heights = $state<Record<string, number>>({});
  const lastTop = new Map<string, number>();

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
    const shown = all.filter((t) => t.kind === 'question');
    const order = [...shown].filter((t) => t.kind === 'question').sort((a, b) => a.createdAt - b.createdAt);
    const sideOf = (t: ThreadModel): 'left' | 'right' =>
      settings.value.questionPlacement === 'both' && !marks && t.kind === 'question' && order.indexOf(t) % 2 === 1 ? 'left' : 'right';

    const anchored: { t: ThreadModel; top: number; orphan: boolean }[] = [];
    for (const t of shown) {
      const g = geometry.byId[t.id];
      if (g === undefined) continue;
      if (g) lastTop.set(t.id, g.top);
      // Editing the quoted words unanchors a thread for as long as they differ; it waits where it was, not at the top.
      anchored.push({ t, top: (g ? g.top : lastTop.get(t.id)) ?? Number.POSITIVE_INFINITY, orphan: g === null });
    }
    anchored.sort((a, b) => a.top - b.top || a.t.createdAt - b.t.createdAt);

    const bottoms = { left: -GAP, right: -GAP };
    const out: Placed[] = [];
    for (const a of anchored) {
      const side = sideOf(a.t);
      const y = Number.isFinite(a.top) ? Math.max(a.top, bottoms[side] + GAP) : bottoms[side] + GAP + 24;
      bottoms[side] = y + (heights[a.t.id] ?? 60);
      out.push({ thread: a.t, side, y, orphan: a.orphan });
    }
    return out;
  });

  /** A thread that just arrived gets the full tether for a moment, unasked. */
  const NEW_MS = 2400;
  let arrivedId = $state<string | null>(null);
  $effect(() => {
    const latest = Object.entries(threads.arrivedAt).sort((a, b) => b[1] - a[1])[0];
    if (!latest) return;
    const left = NEW_MS - (performance.now() - latest[1]);
    if (left <= 0) return;
    arrivedId = latest[0];
    const t = window.setTimeout(() => (arrivedId = null), left);
    return () => window.clearTimeout(t);
  });

  const target = $derived.by(() => {
    const id = threads.openId ?? threads.hoverId ?? arrivedId;
    const p = placed.find((x) => x.thread.id === id && !x.orphan);
    const g = id ? geometry.byId[id] : null;
    if (!p || !g) return null;
    const right = p.side === 'right';
    return {
      id: p.thread.id,
      faint: p.thread.id !== threads.openId && p.thread.id !== arrivedId,
      sx: right ? g.endX + QUOTE_GAP : g.startX - QUOTE_GAP,
      sy: right ? g.endY : g.startY,
      ex: right ? geometry.pageWidth + MARGIN_GAP + BLOT_C - 11 : -MARGIN_GAP - BLOT_C + 11,
      ey: p.y + BLOT_Y,
    };
  });

  const focusId = $derived(
    threads.openId ?? (voice.playing ? (threads.list.find((t) => t.messages.some((m) => m.id === voice.speakingId))?.id ?? null) : null),
  );
</script>

{#if placed.length}
  <div class="margins" class:resizing={layout.dragWidth !== null} style:--mw="{width}px" style:--gap="{MARGIN_GAP}px">
    {#if target}
      {#key `${target.id}:${target.faint}`}
        <Tether sx={target.sx} sy={target.sy} ex={target.ex} ey={target.ey} faint={target.faint} />
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
  /* A thread's menu drops past its slot; later slots would paint over it and take its clicks. */
  .slot:has(:global([role='menu'])) { z-index: 1; }
  .margins.resizing .slot { transition: none; }

  @media (prefers-reduced-motion: reduce) {
    .slot { transition: opacity 340ms var(--ease-step); }
    .margins.resizing .slot { transition: none; }
  }
</style>
