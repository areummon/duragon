import { quintOut } from 'svelte/easing';
import type { TransitionConfig } from 'svelte/transition';

const reduced = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;

interface RiseOptions {
  y?: number;
  scale?: number;
  duration?: number;
}

/** Fade with a small rise. Popovers, menus. */
export function rise(_node: Element, { y = 4, scale = 1, duration = 160 }: RiseOptions = {}): TransitionConfig {
  const still = reduced();
  return {
    duration,
    easing: quintOut,
    css: (t) => (still ? `opacity:${t}` : `opacity:${t};transform:translateY(${(1 - t) * y}px) scale(${scale + (1 - scale) * t})`),
  };
}

/** Height and opacity reveal for blocks that push the content below them. */
export function unfold(node: Element, { duration = 260 }: { duration?: number } = {}): TransitionConfig {
  const height = node.getBoundingClientRect().height;
  return {
    duration: reduced() ? 0 : duration,
    easing: quintOut,
    css: (t) => `overflow:hidden;height:${height * t}px;opacity:${t}`,
  };
}

/** CSS-style cubic-bezier as a Svelte easing function. */
function bezier(x1: number, y1: number, x2: number, y2: number) {
  const cx = 3 * x1, bx = 3 * (x2 - x1) - cx, ax = 1 - cx - bx;
  const cy = 3 * y1, by = 3 * (y2 - y1) - cy, ay = 1 - cy - by;
  const x = (s: number) => ((ax * s + bx) * s + cx) * s;
  const y = (s: number) => ((ay * s + by) * s + cy) * s;
  return (t: number) => {
    let s = t;
    for (let i = 0; i < 8; i++) {
      const err = x(s) - t;
      const d = (3 * ax * s + 2 * bx) * s + cx;
      if (Math.abs(err) < 1e-5 || Math.abs(d) < 1e-6) break;
      s -= err / d;
    }
    return y(Math.min(1, Math.max(0, s)));
  };
}

const easeStep = bezier(0.22, 1, 0.36, 1);

/** Modal card: fade with a scale from .98. */
export function pop(_node: Element, { duration = 200, from = 0.98 }: { duration?: number; from?: number } = {}): TransitionConfig {
  const still = reduced();
  return {
    duration,
    easing: easeStep,
    css: (t) => (still ? `opacity:${t}` : `opacity:${t};transform:scale(${from + (1 - from) * t})`),
  };
}

/** Backdrop fade. */
export function veil(_node: Element, { duration = 200 }: { duration?: number } = {}): TransitionConfig {
  return { duration, easing: easeStep, css: (t) => `opacity:${t}` };
}
