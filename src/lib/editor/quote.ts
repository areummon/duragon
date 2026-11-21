import type { Node as PMNode } from '@tiptap/pm/model';

export interface Range {
  from: number;
  to: number;
}

/** The doc's text with blocks joined by "\n" (like the plain text the AI reads), and where each text node sits. */
export interface TextIndex {
  text: string;
  /** One per text node, in order: its offset in `text` and its ProseMirror position. */
  runs: { at: number; pos: number; len: number }[];
}

export function indexText(doc: PMNode): TextIndex {
  const parts: string[] = [];
  const runs: TextIndex['runs'] = [];
  let at = 0;
  let first = true;
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    if (!first) {
      parts.push('\n');
      at += 1;
    }
    first = false;
    node.forEach((child, offset) => {
      if (!child.isText) return;
      const t = child.text!;
      parts.push(t);
      runs.push({ at, pos: pos + 1 + offset, len: t.length });
      at += t.length;
    });
    return false;
  });
  return { text: parts.join(''), runs };
}

/** The ProseMirror position of character `i` of the index text, or -1 for a block separator. */
function posOf(index: TextIndex, i: number): number {
  const { runs } = index;
  let lo = 0;
  let hi = runs.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const r = runs[mid];
    if (i < r.at) hi = mid - 1;
    else if (i >= r.at + r.len) lo = mid + 1;
    else return r.pos + (i - r.at);
  }
  return -1;
}

function range(index: TextIndex, at: number, len: number): Range | null {
  const from = posOf(index, at);
  const to = posOf(index, at + len - 1) + 1;
  return from >= 0 && to > from ? { from, to } : null;
}

/** Where `quote` first occurs. */
export function locate(index: TextIndex, quote: string): Range | null {
  const at = quote ? index.text.indexOf(quote) : -1;
  return at < 0 ? null : range(index, at, quote.length);
}

const blank = (s: string) => s.trim() === '';

/**
 * Where the `nth` copy (0-based) of `line` is, counting only copies that fill a whole line, or null when none does.
 * With fewer copies than that, the last one: a thread whose copy of a repeated line was deleted still shows, on the
 * copy that remains.
 */
export function locateLine(index: TextIndex, line: string, nth: number): Range | null {
  const { text } = index;
  let found = -1;
  for (let at = line ? text.indexOf(line) : -1, n = 0; at >= 0 && n <= nth; at = text.indexOf(line, at + 1)) {
    const end = at + line.length;
    const lineEnd = text.indexOf('\n', end);
    if (!blank(text.slice(text.lastIndexOf('\n', at - 1) + 1, at)) || !blank(text.slice(end, lineEnd < 0 ? undefined : lineEnd))) continue;
    found = at;
    n++;
  }
  return found < 0 ? null : range(index, found, line.length);
}

/** Finds `quote` in the doc's text. Returns ProseMirror positions, or null when the quote is not there. */
export function findQuote(doc: PMNode, quote: string): Range | null {
  return locate(indexText(doc), quote);
}
