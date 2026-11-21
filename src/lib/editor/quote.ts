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

/**
 * Where the `nth` occurrence of `quote` is (0-based). With fewer occurrences than that, the last one: a thread whose
 * copy of a repeated line was deleted still shows, on the copy that remains.
 */
export function locate(index: TextIndex, quote: string, nth = 0): Range | null {
  if (!quote) return null;
  let at = -1;
  for (let i = 0, from = 0; i <= nth; i++) {
    const next = index.text.indexOf(quote, from);
    if (next < 0) break;
    at = next;
    from = next + 1;
  }
  if (at < 0) return null;
  const from = posOf(index, at);
  const to = posOf(index, at + quote.length - 1) + 1;
  return from >= 0 && to > from ? { from, to } : null;
}

/** Finds `quote` in the doc's text. Returns ProseMirror positions, or null when the quote is not there. */
export function findQuote(doc: PMNode, quote: string): Range | null {
  return locate(indexText(doc), quote);
}
