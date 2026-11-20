const DURATION = 260;
const ID = 'reflow';

interface Spot {
  x: number;
  y: number;
}

interface ReflowOptions {
  /** Items pushed along their line by an auto margin follow the edge instantly; they only glide when they change line. */
  tracksEdge?: (el: HTMLElement) => boolean;
  onLayout?: () => void;
}

/** Glides a wrapping row's children to their new places when the row reflows, so items pass between lines one at a time. */
export function reflow(row: HTMLElement, options: ReflowOptions = {}) {
  let opts = options;
  const spots = new Map<Element, Spot>();
  const ease = getComputedStyle(document.documentElement).getPropertyValue('--ease-step').trim() || 'ease-out';

  function update() {
    const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
    for (const child of row.children) {
      const el = child as HTMLElement;
      const was = spots.get(el);
      const now = { x: el.offsetLeft, y: el.offsetTop };
      spots.set(el, now);
      if (!was || still) continue;
      const sameLine = was.y === now.y;
      if (sameLine && was.x === now.x) continue;
      if (sameLine && opts.tracksEdge?.(el)) continue;

      let tx = 0;
      let ty = 0;
      const running = el.getAnimations().find((a) => a.id === ID);
      if (running) {
        const m = new DOMMatrix(getComputedStyle(el).transform);
        tx = m.m41;
        ty = m.m42;
        running.cancel();
      }
      const dx = was.x + tx - now.x;
      const dy = was.y + ty - now.y;
      if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
      const a = el.animate([{ transform: `translate(${dx}px, ${dy}px)` }, { transform: 'none' }], { duration: DURATION, easing: ease });
      a.id = ID;
    }
    opts.onLayout?.();
  }

  const ro = new ResizeObserver(update);
  const watch = () => {
    ro.disconnect();
    ro.observe(row);
    for (const child of row.children) ro.observe(child);
  };
  const mo = new MutationObserver(() => {
    watch();
    update();
  });
  watch();
  mo.observe(row, { childList: true });

  return {
    update(next: ReflowOptions) {
      opts = next;
    },
    destroy() {
      ro.disconnect();
      mo.disconnect();
    },
  };
}
