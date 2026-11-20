import type { Node as PMNode } from '@tiptap/pm/model';

export interface Range {
  from: number;
  to: number;
}

/**
 * Finds `quote` in the doc's text, where blocks are joined with "\n" like the plain text the
 * AI reads. Returns ProseMirror positions, or null when the quote is not there.
 */
export function findQuote(doc: PMNode, quote: string): Range | null {
  if (!quote) return null;
  let text = '';
  const positions: number[] = [];
  let first = true;
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    if (!first) {
      text += '\n';
      positions.push(-1);
    }
    first = false;
    node.forEach((child, offset) => {
      if (!child.isText) return;
      for (let i = 0; i < child.text!.length; i++) {
        text += child.text![i];
        positions.push(pos + 1 + offset + i);
      }
    });
    return false;
  });
  const at = text.indexOf(quote);
  if (at < 0) return null;
  const from = positions[at];
  const to = positions[at + quote.length - 1] + 1;
  return from >= 0 && to > from ? { from, to } : null;
}
