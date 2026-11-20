function hash(seed: string): number {
  let h = 2166136261;
  for (let i = 0; i < seed.length; i++) h = Math.imul(h ^ seed.charCodeAt(i), 16777619);
  return h >>> 0;
}

function rng(seed: string) {
  let a = hash(seed) || 1;
  return () => {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const SEGMENTS = 30;
const RADIUS = 7.5;
const cache = new Map<string, string>();

/** ~30 quadratic segments around a circle with seeded radius jitter, in a -13..13 box. */
export function blotPath(seed: string): string {
  const hit = cache.get(seed);
  if (hit) return hit;
  const rand = rng(seed);
  const pts: [number, number][] = [];
  for (let i = 0; i < SEGMENTS; i++) {
    const a = (i / SEGMENTS) * Math.PI * 2;
    const r = RADIUS * (1 + (rand() - 0.5) * 0.42);
    pts.push([r * Math.cos(a), r * Math.sin(a)]);
  }
  const f = (n: number) => n.toFixed(2);
  const mid = (a: [number, number], b: [number, number]) => [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2] as const;
  const start = mid(pts[SEGMENTS - 1], pts[0]);
  let d = `M${f(start[0])} ${f(start[1])}`;
  for (let i = 0; i < SEGMENTS; i++) {
    const p = pts[i];
    const m = mid(p, pts[(i + 1) % SEGMENTS]);
    d += `Q${f(p[0])} ${f(p[1])} ${f(m[0])} ${f(m[1])}`;
  }
  d += 'Z';
  cache.set(seed, d);
  return d;
}

/** Static markup for widget decorations that cannot mount a component. */
export function blotSvg(seed: string, size: number, cls = ''): string {
  return `<svg class="blot ${cls}" width="${size}" height="${size}" viewBox="-13 -13 26 26" aria-hidden="true"><path fill="currentColor" d="${blotPath(seed)}"/></svg>`;
}
