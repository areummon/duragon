const WINDOW = 80;
const LIFT = 60;
const TAU = 325;
const MIN_FLICK = 0.3;
const MAX_FLICK = 10;
const STOP = 0.02;

const page = () => document.scrollingElement ?? document.documentElement;

function scrollable(el: Element): boolean {
  if (el.scrollHeight <= el.clientHeight + 1) return false;
  return /auto|scroll/.test(getComputedStyle(el).overflowY);
}

function canMove(el: Element, dy: number): boolean {
  return dy < 0 ? el.scrollTop > 0 : el.scrollTop + el.clientHeight < el.scrollHeight - 1;
}

/**
 * The element a gesture scrolls, chosen the way browsers latch: the innermost scroller under the pointer that can still
 * move this way, else one that contains its overscroll, else the page.
 */
function targetFor(e: WheelEvent): Element {
  for (const node of e.composedPath()) {
    if (!(node instanceof Element) || node === document.body || node === document.documentElement) break;
    if (!scrollable(node)) continue;
    if (canMove(node, e.deltaY)) return node;
    if (/contain|none/.test(getComputedStyle(node).overscrollBehaviorY)) return node;
  }
  return page();
}

/**
 * Scrolls straight from touchpad input, scaled to the distance the finger moved, and glides to a stop after a flick.
 * Covers the page and every panel that scrolls (chat, sidebar, dialogs). Used where the engine's own wheel handling
 * overshoots and eases (WebKitGTK through XWayland).
 */
export function scaleWheel(scale: number) {
  if (scale === 1) return;
  let target: Element | null = null;
  let pending = 0;
  let carry = 0;
  let velocity = 0;
  let lastInput = -Infinity;
  let lastFrame = 0;
  let frame = 0;
  let recent: { t: number; d: number }[] = [];

  /** Moves the target by whole pixels and keeps the fraction, so slow glides do not stall on rounding. */
  function move(d: number): boolean {
    if (!target) return false;
    carry += d;
    const whole = Math.trunc(carry);
    if (!whole) return true;
    carry -= whole;
    const before = target.scrollTop;
    target.scrollTop = before + whole;
    return target.scrollTop !== before;
  }

  function step(now: number) {
    frame = 0;
    const dt = lastFrame ? Math.min(now - lastFrame, 50) : 16;
    lastFrame = now;
    if (pending) {
      move(pending);
      pending = 0;
    }
    if (recent.length && now - lastInput > LIFT) {
      const span = Math.max(lastInput - recent[0].t, 16);
      const v = recent.reduce((sum, r) => sum + r.d, 0) / span;
      recent = [];
      velocity = Math.abs(v) >= MIN_FLICK ? Math.sign(v) * Math.min(Math.abs(v), MAX_FLICK) : 0;
    }
    if (velocity) {
      const moved = move(velocity * dt);
      velocity *= Math.exp(-dt / TAU);
      if (Math.abs(velocity) < STOP || !moved) velocity = 0;
    }
    if (velocity || recent.length) frame = requestAnimationFrame(step);
    else lastFrame = 0;
  }

  addEventListener(
    'wheel',
    (e) => {
      if (e.ctrlKey || e.deltaMode !== 0 || Math.abs(e.deltaX) > Math.abs(e.deltaY)) return;
      e.preventDefault();
      const fresh = e.timeStamp - lastInput > LIFT;
      if (fresh || !target?.isConnected) {
        target = targetFor(e);
        carry = 0;
        recent = [];
      }
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
