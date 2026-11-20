<script lang="ts">
  interface Props {
    /** Start beside the quote and end at the thread's blot, in page coordinates. */
    sx: number;
    sy: number;
    ex: number;
    ey: number;
    /** Hover only: lighter, thinner and quicker than the tie of an open or new thread. */
    faint: boolean;
  }
  let { sx, sy, ex, ey, faint }: Props = $props();

  const PAD = 24;
  const uid = $props.id();

  const d = $derived.by(() => {
    const dx = ex - sx;
    return `M${sx} ${sy}C${sx + 0.6 * dx} ${sy} ${ex - 0.6 * dx} ${ey} ${ex} ${ey}`;
  });
  const box = $derived({
    x: Math.min(sx, ex) - PAD,
    y: Math.min(sy, ey) - PAD,
    width: Math.abs(ex - sx) + 2 * PAD,
    height: Math.abs(ey - sy) + 2 * PAD,
  });
</script>

<svg class="thread-tether" class:faint aria-hidden="true">
  <filter id="rough-{uid}" filterUnits="userSpaceOnUse" {...box}>
    <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="2" seed="3" result="wobble" />
    <feDisplacementMap in="SourceGraphic" in2="wobble" scale="2.2" xChannelSelector="R" yChannelSelector="G" result="rough" />
    <feTurbulence type="fractalNoise" baseFrequency="1.7" numOctaves="1" seed="9" result="grain" />
    <feColorMatrix in="grain" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  -2.4 0 0 0 1.75" result="holes" />
    <feComposite in="rough" in2="holes" operator="in" />
  </filter>
  <mask id="reveal-{uid}" maskUnits="userSpaceOnUse" {...box}>
    <path class="reveal" {d} pathLength="1" />
  </mask>
  <g mask="url(#reveal-{uid})">
    <g filter="url(#rough-{uid})">
      <path class="tie" {d} />
    </g>
  </g>
</svg>

<style>
  .thread-tether {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
  }
  .tie {
    fill: none;
    stroke: var(--riddle);
    stroke-linecap: round;
    stroke-width: 1.8px;
    stroke-opacity: 0.42;
  }
  .faint .tie { stroke-width: 1.5px; stroke-opacity: 0.26; }
  .reveal {
    fill: none;
    stroke: #fff;
    stroke-width: 10px;
    stroke-linecap: round;
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    animation: tether-draw 560ms var(--ease-tether) 140ms forwards;
  }
  .faint .reveal { animation-duration: 320ms; animation-delay: 60ms; }

  @media (prefers-reduced-motion: reduce) {
    .reveal { stroke-dashoffset: 0; animation: none; }
  }
</style>
