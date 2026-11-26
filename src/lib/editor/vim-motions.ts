import type { Node as PMNode, ResolvedPos } from '@tiptap/pm/model';
import { Selection, type EditorState } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';
import { isEmptyLine } from './create';

interface TextblockSpan {
  start: number;
  end: number;
  node: PMNode;
}

function blockText(node: PMNode): string {
  return node.textBetween(0, node.content.size, '\n', '\ufffc');
}

export function parentText(resolved: ResolvedPos): string {
  return blockText(resolved.parent);
}

function charClass(ch: string): 'space' | 'word' | 'punct' {
  if (/\s/.test(ch)) return 'space';
  if (/[A-Za-z0-9_]/.test(ch)) return 'word';
  return 'punct';
}

export function firstNonSpacePos(resolved: ResolvedPos): number {
  const start = resolved.start(resolved.depth);
  const idx = parentText(resolved).search(/\S/);
  return idx === -1 ? start : start + idx;
}

function collectTextblocks(doc: PMNode): TextblockSpan[] {
  const blocks: TextblockSpan[] = [];
  doc.descendants((node, pos) => {
    if (node.isTextblock) {
      blocks.push({ start: pos + 1, end: pos + 1 + node.content.size, node });
      return false;
    }
    return true;
  });
  return blocks;
}

function adjacentTextblock(doc: PMNode, pos: number, dir: 1 | -1): TextblockSpan | null {
  const blocks = collectTextblocks(doc);
  const idx = blocks.findIndex((b) => pos >= b.start && pos <= b.end);
  if (idx === -1) return blocks[0] ?? null;
  return blocks[idx + dir] ?? null;
}

function scanWordInBlock(text: string, startIdx: number, dir: 1 | -1): number | null {
  const inBounds = (i: number) => i >= 0 && i < text.length;
  let i = startIdx;
  while (inBounds(i) && charClass(text[i]) === 'space') i += dir;
  if (!inBounds(i)) return null;
  const cls = charClass(text[i]);
  while (inBounds(i + dir) && charClass(text[i + dir]) === cls) i += dir;
  return i;
}

function scanWord(doc: PMNode, pos: number, dir: 1 | -1): number {
  const resolved = doc.resolve(pos);
  const start = resolved.start(resolved.depth);
  const end = resolved.end(resolved.depth);
  const hit = scanWordInBlock(parentText(resolved), resolved.parentOffset + dir, dir);
  if (hit !== null) return start + hit;

  const adj = adjacentTextblock(doc, pos, dir);
  if (!adj) return dir > 0 ? end : start;
  const adjText = blockText(adj.node);
  const adjHit = scanWordInBlock(adjText, dir > 0 ? 0 : adjText.length - 1, dir);
  return adjHit === null ? adj.start : adj.start + adjHit;
}

export function nextWordStart(doc: PMNode, pos: number, crossBlock: boolean): number {
  const resolved = doc.resolve(pos);
  const start = resolved.start(resolved.depth);
  const end = resolved.end(resolved.depth);
  const text = parentText(resolved);
  let i = resolved.parentOffset;

  if (i < text.length) {
    const cls = charClass(text[i]);
    if (cls !== 'space') {
      while (i < text.length && charClass(text[i]) === cls) i++;
    }
    while (i < text.length && charClass(text[i]) === 'space') i++;
    if (i < text.length) return start + i;
  }

  if (!crossBlock) return end;
  const next = adjacentTextblock(doc, pos, 1);
  if (!next) return end;
  const first = blockText(next.node).search(/\S/);
  return first === -1 ? next.start : next.start + first;
}

export function changeWordEnd(doc: PMNode, pos: number): number {
  const resolved = doc.resolve(pos);
  const start = resolved.start(resolved.depth);
  const end = resolved.end(resolved.depth);
  const text = parentText(resolved);
  let i = resolved.parentOffset;
  if (i >= text.length) return end;

  const cls = charClass(text[i]);
  while (i < text.length && charClass(text[i]) === cls) i++;
  return start + i;
}

function verticalMove(view: EditorView, pos: number, dir: 1 | -1): number {
  const { doc } = view.state;
  const resolved = doc.resolve(pos);
  const blockStart = resolved.start(resolved.depth);
  const blockEnd = resolved.end(resolved.depth);

  // Try stepping one visual line inside a wrapped textblock first.
  try {
    const cur = view.coordsAtPos(pos);
    const probeTop = dir > 0 ? cur.bottom + 4 : cur.top - 4;
    const hit = view.posAtCoords({ left: cur.left, top: probeTop });
    if (hit && hit.pos >= blockStart && hit.pos <= blockEnd && hit.pos !== pos) {
      const target = view.coordsAtPos(hit.pos);
      if (dir > 0 ? target.top > cur.top + 2 : target.bottom < cur.bottom - 2) {
        return hit.pos;
      }
    }
  } catch {
    // Fall back to block-level movement if coordinates are unavailable.
  }

  const adj = adjacentTextblock(doc, pos, dir);
  if (!adj) return pos;
  return Math.min(adj.end, adj.start + resolved.parentOffset);
}

export function topOfDoc(state: EditorState): number {
  return Selection.atStart(state.doc).from;
}

export function bottomOfDoc(state: EditorState): number {
  const { doc } = state;
  const blocks = collectTextblocks(doc);
  if (blocks.length === 0) return Selection.atEnd(doc).from;

  // Respect Duragon's trailing LastLine empty paragraph.
  let lastIdx = blocks.length - 1;
  if (lastIdx > 1 && isEmptyLine(blocks[lastIdx].node)) {
    lastIdx--;
  }
  // Walk back to the last non-empty body block when possible.
  for (let i = lastIdx; i >= 1; i--) {
    if (blocks[i].node.content.size > 0) return blocks[i].start;
  }
  return blocks[lastIdx].start;
}

interface MotionContext {
  view: EditorView;
  pos: number;
  resolved: ResolvedPos;
  start: number;
  end: number;
}

const MOTIONS: Record<string, (ctx: MotionContext) => number> = {
  h: ({ start, pos }) => Math.max(start, pos - 1),
  l: ({ end, pos }) => Math.min(end, pos + 1),
  j: ({ view, pos }) => verticalMove(view, pos, 1),
  k: ({ view, pos }) => verticalMove(view, pos, -1),
  w: ({ view, pos }) => nextWordStart(view.state.doc, pos, true),
  b: ({ view, pos }) => scanWord(view.state.doc, pos, -1),
  e: ({ view, pos }) => scanWord(view.state.doc, pos, 1),
  '0': ({ start }) => start,
  '^': ({ resolved }) => firstNonSpacePos(resolved),
  $: ({ end }) => end,
  G: ({ view }) => bottomOfDoc(view.state),
};

export function resolveMotion(view: EditorView, pos: number, key: string): number | null {
  const motion = MOTIONS[key];
  if (!motion) return null;
  const resolved = view.state.doc.resolve(pos);
  return motion({
    view,
    pos,
    resolved,
    start: resolved.start(resolved.depth),
    end: resolved.end(resolved.depth),
  });
}
