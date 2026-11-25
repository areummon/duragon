import type { EditorView } from '@tiptap/pm/view';
import type { Range } from './quote';

/** One visual line of a passage, in page coordinates: x span, baseline y and middle. */
export type PencilLine = { x1: number; x2: number; y: number; mid: number };

const SAME_LINE = 6;
const MIN_RECT = 2;

/** Client rects of the text in `range`, merged per visual line, relative to `page`. Widgets are skipped. */
export function measureLines(view: EditorView, range: Range, page: HTMLElement, pageBox?: DOMRect): PencilLine[] {
  const start = view.domAtPos(range.from);
  const end = view.domAtPos(range.to);
  const whole = document.createRange();
  whole.setStart(start.node, start.offset);
  whole.setEnd(end.node, end.offset);

  const rects: DOMRect[] = [];
  const walker = document.createTreeWalker(whole.commonAncestorContainer, NodeFilter.SHOW_TEXT);
  for (let n = walker.currentNode as Node | null; n; n = walker.nextNode()) {
    if (n.nodeType !== Node.TEXT_NODE || !whole.intersectsNode(n)) continue;
    if (n.parentElement?.closest('[contenteditable="false"]')) continue;
    const part = document.createRange();
    part.selectNodeContents(n);
    if (n === whole.startContainer) part.setStart(n, whole.startOffset);
    if (n === whole.endContainer) part.setEnd(n, whole.endOffset);
    for (const r of part.getClientRects()) if (r.width >= MIN_RECT) rects.push(r);
  }

  const merged: { left: number; right: number; top: number; bottom: number }[] = [];
  for (const r of rects) {
    const line = merged.find((m) => Math.abs(m.bottom - r.bottom) <= SAME_LINE);
    if (line) {
      line.left = Math.min(line.left, r.left);
      line.right = Math.max(line.right, r.right);
      line.top = Math.min(line.top, r.top);
      line.bottom = Math.max(line.bottom, r.bottom);
    } else merged.push({ left: r.left, right: r.right, top: r.top, bottom: r.bottom });
  }
  const box = pageBox ?? page.getBoundingClientRect();
  return merged
    .sort((a, b) => a.bottom - b.bottom)
    .map((m) => ({
      x1: m.left - box.left,
      x2: m.right - box.left,
      y: m.bottom - box.top + 1,
      mid: (m.top + m.bottom) / 2 - box.top,
    }));
}

const f = (n: number) => n.toFixed(2);

/** A wobbly underline under one line, seeded so it stays put across re-measures. */
export function underlinePath(l: PencilLine, seed: number): string {
  const w = l.x2 - l.x1;
  const n = Math.max(2, Math.round(w / 34));
  const o = (k: number) => 0.75 * Math.sin(12.9898 * seed + 78.233 * k);
  let d = `M${f(l.x1)} ${f(l.y + o(0))}`;
  for (let k = 1; k <= n; k++) {
    const xk = l.x1 + (w * k) / n;
    const cx = xk - w / n / 2;
    const tail = k === n ? -0.9 : 0;
    d += `Q${f(cx)} ${f(l.y + 1.3 * o(k + 0.5))} ${f(xk)} ${f(l.y + o(k) + tail)}`;
  }
  return d;
}

/** A single vertical stroke in the left gutter, for passages of four lines or more. */
export function gutterPath(lines: PencilLine[]): string {
  const first = lines[0];
  const last = lines[lines.length - 1];
  const x = Math.min(...lines.map((l) => l.x1)) - 14;
  const step = (last.y - first.y) / Math.max(1, lines.length - 1);
  const top = first.y - 0.85 * step;
  const bottom = last.y - 2;
  const n = Math.max(2, Math.round((bottom - top) / 40));
  const l = (k: number) => 0.7 * Math.sin(78.233 * k + 4.1);
  let d = `M${f(x + l(0))} ${f(top)}`;
  for (let k = 1; k <= n; k++) {
    const t = top + ((bottom - top) * k) / n;
    const c = t - (bottom - top) / n / 2;
    d += `Q${f(x + 1.2 * l(k + 0.5))} ${f(c)} ${f(x + l(k) + (k === n ? 0.8 : 0))} ${f(t)}`;
  }
  return d;
}

export const GUTTER_FROM = 4;
const MIN_SPEED = 650;

/** Draw duration and start delay per line (seconds), so the hand moves along the lines in turn. */
export function strokeTiming(lines: PencilLine[]): { draw: number; delay: number }[] {
  const speed = Math.max(MIN_SPEED, lines.reduce((s, l) => s + l.x2 - l.x1, 0) / 1.6);
  let at = 0;
  return lines.map((l) => {
    const draw = (l.x2 - l.x1) / speed;
    const t = { draw, delay: at };
    at += draw;
    return t;
  });
}
