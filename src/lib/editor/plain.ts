import type { Node as PMNode } from '@tiptap/pm/model';

interface JsonNode {
  type: string;
  text?: string;
  attrs?: Record<string, unknown>;
  content?: JsonNode[];
}

const isTextblock = (n: JsonNode) => n.type === 'paragraph' || n.type === 'heading' || n.type === 'title' || n.type === 'codeBlock';

function inlineText(n: JsonNode): string {
  if (n.type === 'text') return n.text ?? '';
  if (n.type === 'hardBreak') return '\n';
  return (n.content ?? []).map(inlineText).join('');
}

function blockLines(n: JsonNode, out: string[]) {
  if (isTextblock(n)) out.push(inlineText(n));
  else n.content?.forEach((c) => blockLines(c, out));
}

export function blockTexts(doc: unknown): string[] {
  const root = doc as JsonNode;
  return (root.content ?? []).map((b) => {
    const lines: string[] = [];
    blockLines(b, lines);
    return lines.join('\n');
  });
}

/** One top-level block's text, as `blockTexts` reads it. */
export const blockText = (block: PMNode) => block.textBetween(0, block.content.size, '\n', '\n');

/** `blockTexts` of the live editor doc, without serializing it: each top-level block's text, title first. */
export function docBlocks(doc: PMNode): string[] {
  const out: string[] = [];
  doc.forEach((block) => out.push(blockText(block)));
  return out;
}

/**
 * Which blocks of `after` (by index, never the title) hold text that is new since `before`. Blocks that match from
 * either end are unchanged, so a pasted copy of a line is the new one rather than the copy that was already there.
 * Between them, text that was there before is matched up and not counted.
 */
export function changedBlocks(before: string[], after: string[]): number[] {
  let start = 0;
  while (start < before.length && start < after.length && before[start] === after[start]) start++;
  let end = 0;
  const room = Math.min(before.length, after.length) - start;
  while (end < room && before[before.length - 1 - end] === after[after.length - 1 - end]) end++;
  const left = new Map<string, number>();
  for (const t of before.slice(start, before.length - end)) left.set(t, (left.get(t) ?? 0) + 1);
  const out: number[] = [];
  for (let i = start; i < after.length - end; i++) {
    const n = left.get(after[i]) ?? 0;
    if (n > 0) left.set(after[i], n - 1);
    else if (i > 0 && after[i].trim() !== '') out.push(i);
  }
  return out;
}

/** Title, then each top-level block's text, one per line. What AI tasks and search read. */
export function plainText(doc: unknown): string {
  const [title = '', ...body] = blockTexts(doc);
  return [title, ...body].join('\n');
}

export function docTitle(doc: unknown): string {
  return blockTexts(doc)[0] ?? '';
}

const isEmptyLine = (n: JsonNode | undefined) => n?.type === 'paragraph' && !n.content?.length;

/** Makes sure the doc starts with a title block and ends with the empty line the editor keeps there. */
export function normalizeDoc(doc: unknown, title: string): JsonNode {
  const root = (doc as JsonNode | null) ?? { type: 'doc', content: [] };
  const content = [...(root.content ?? [])];
  if (content[0]?.type !== 'title') {
    content.unshift({ type: 'title', content: title ? [{ type: 'text', text: title }] : [] });
  }
  if (!isEmptyLine(content.at(-1))) content.push({ type: 'paragraph' });
  return { ...root, type: 'doc', content };
}

/** The doc as it is saved: without the editor's empty last line, unless that is the whole body. */
export function withoutLastLine<T extends { content?: unknown[] }>(doc: T): T {
  const content = doc.content ?? [];
  return content.length > 2 && isEmptyLine(content.at(-1) as JsonNode) ? { ...doc, content: content.slice(0, -1) } : doc;
}

function md(n: JsonNode, indent: string, out: string[], counter?: { n: number }) {
  switch (n.type) {
    case 'title': out.push(`# ${inlineText(n)}`, ''); return;
    case 'heading': out.push(`${'#'.repeat(Number(n.attrs?.level ?? 1) + 0)} ${inlineText(n)}`, ''); return;
    case 'paragraph': out.push(indent + inlineText(n), ''); return;
    case 'codeBlock': out.push('```' + (n.attrs?.language ?? ''), inlineText(n), '```', ''); return;
    case 'horizontalRule': out.push('---', ''); return;
    case 'blockquote': {
      const inner: string[] = [];
      n.content?.forEach((c) => md(c, '', inner));
      out.push(...inner.filter((l, i, a) => !(l === '' && i === a.length - 1)).map((l) => `> ${l}`.trimEnd()), '');
      return;
    }
    case 'bulletList':
    case 'orderedList': {
      const c = { n: Number(n.attrs?.start ?? 1) };
      n.content?.forEach((li) => md(li, indent, out, n.type === 'orderedList' ? c : undefined));
      out.push('');
      return;
    }
    case 'listItem': {
      const bullet = counter ? `${counter.n++}. ` : '- ';
      const [first, ...rest] = n.content ?? [];
      out.push(indent + bullet + (first ? inlineText(first) : ''));
      rest.forEach((c) => {
        if (c.type === 'bulletList' || c.type === 'orderedList') {
          const nested: string[] = [];
          md(c, indent + '  ', nested);
          out.push(...nested.filter((l) => l !== ''));
        } else out.push(indent + '  ' + inlineText(c));
      });
      return;
    }
    default: n.content?.forEach((c) => md(c, indent, out));
  }
}

/** Markdown-flavoured plain text for the clipboard. Marks are dropped. */
export function markdownish(doc: unknown): string {
  const out: string[] = [];
  ((doc as JsonNode).content ?? []).forEach((b) => md(b, '', out));
  return out.join('\n').replace(/\n{3,}/g, '\n\n').trim() + '\n';
}

/** Where block `i` begins in the note's text: its blocks joined by "\n", as the backend joins them. */
export const blockStart = (texts: string[], i: number) => texts.slice(0, i).reduce((n, t) => n + t.length + 1, 0);

/**
 * Which occurrence of `target`, from 0, starts at `at` in the note's text, counted without overlaps the way the backend
 * finds it, so a long note is cut around that copy.
 */
export function occurrence(texts: string[], at: number, target: string): number {
  const plain = texts.join('\n');
  let n = 0;
  for (let k = plain.indexOf(target); k >= 0 && k < at; k = plain.indexOf(target, k + target.length)) n++;
  return n;
}

/** Which occurrence of its text the line in block `i` is. */
export function lineOccurrence(texts: string[], i: number): number {
  const line = texts[i].trim();
  return occurrence(texts, blockStart(texts, i) + texts[i].indexOf(line), line);
}

/** Which occurrence of `text` the selection starting at `from` is. */
export function selectionOccurrence(doc: PMNode, from: number, text: string): number {
  const texts = docBlocks(doc);
  const $from = doc.resolve(from);
  const into = $from.depth ? doc.textBetween($from.start(1), from, '\n', '\n').length : 0;
  const lead = text.length - text.trimStart().length;
  return occurrence(texts, blockStart(texts, $from.index(0)) + into + lead, text.trim());
}
