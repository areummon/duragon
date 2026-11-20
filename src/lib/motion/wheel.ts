const WINDOW = 80;
const LIFT = 60;
const TAU = 325;
const MIN_FLICK = 0.3;
const MAX_FLICK = 10;
const STOP = 0.02;

/** True when the wheel is over a panel that scrolls on its own (chat, sidebar, dialogs); those keep native scrolling. */
function overInnerScroller(e: WheelEvent): boolean {
  for (const node of e.composedPath()) {
    if (!(node instanceof HTMLElement) || node === document.body || node === document.documentElement) break;
    if (node.scrollHeight > node.clientHeight && /auto|scroll/.test(getComputedStyle(node).overflowY)) return true;
  }
  return false;
}

/**
 * Scrolls the page straight from touchpad input, scaled to the distance the finger moved, and glides to a stop after a
 * flick. Used where the engine's own wheel handling overshoots and eases (WebKitGTK through XWayland).
 */
export function scaleWheel(scale: number) {
  if (scale === 1) return;
  let pending = 0;
  let velocity = 0;
  let lastInput = 0;
  let lastFrame = 0;
  let frame = 0;
  let recent: { t: number; d: number }[] = [];

  function step(now: number) {
    frame = 0;
    const dt = lastFrame ? Math.min(now - lastFrame, 50) : 16;
    lastFrame = now;
    if (pending) {
      scrollBy(0, pending);
      pending = 0;
    }
    if (recent.length && now - lastInput > LIFT) {
      const span = Math.max(lastInput - recent[0].t, 16);
      const v = recent.reduce((sum, r) => sum + r.d, 0) / span;
      recent = [];
      velocity = Math.abs(v) >= MIN_FLICK ? Math.sign(v) * Math.min(Math.abs(v), MAX_FLICK) : 0;
    }
    if (velocity) {
      const before = scrollY;
      scrollBy(0, velocity * dt);
      velocity *= Math.exp(-dt / TAU);
      if (Math.abs(velocity) < STOP || scrollY === before) velocity = 0;
    }
    if (velocity || recent.length) frame = requestAnimationFrame(step);
    else lastFrame = 0;
  }

  addEventListener(
    'wheel',
    (e) => {
      if (e.ctrlKey || e.deltaMode !== 0 || Math.abs(e.deltaX) > Math.abs(e.deltaY) || overInnerScroller(e)) return;
      e.preventDefault();
      const d = e.deltaY * scale;
      pending += d;
      velocity = 0;
      lastInput = e.timeStamp;
      recent.push({ t: e.timeStamp, d });
      recent = recent.filter((r) => e.timeStamp - r.t <= WINDOW);
      frame ||= requestAnimationFrame(step);
    },
    { passive: false },
  );
}
