import { Extension } from '@tiptap/core';
import type { Node as PMNode } from '@tiptap/pm/model';
import { type EditorState, Plugin, PluginKey, TextSelection } from '@tiptap/pm/state';
import { Decoration, DecorationSet, type EditorView } from '@tiptap/pm/view';
import { ensureKatex, findMath, isKatexReady, type MathRange, onKatexReady, renderTex } from '../rich';

/** The title names the note in the sidebar as plain text, and code is code; neither renders math. */
const PLAIN = new Set(['title', 'codeBlock']);
const OPAQUE = '\uFFFC';

interface MathPluginState {
  deco: DecorationSet;
  ranges: { from: number; to: number }[];
}

const mathKey = new PluginKey<MathPluginState>('duragon-math');

/** Math in one textblock, offsets into its content; inline-code text and inline nodes cannot be part of a formula. */
function blockMath(block: PMNode): MathRange[] {
  let text = '';
  block.forEach((child) => {
    if (!child.isText) text += OPAQUE;
    else if (child.marks.some((m) => m.type.name === 'code')) text += OPAQUE.repeat(child.text!.length);
    else text += child.text;
  });
  return findMath(text).filter((m) => !m.tex.includes(OPAQUE));
}

// Blocks are immutable, so an unchanged block keeps its node object and is never parsed twice.
const parsed = new WeakMap<PMNode, MathRange[]>();
function mathIn(block: PMNode): MathRange[] {
  let ranges = parsed.get(block);
  if (!ranges) parsed.set(block, (ranges = blockMath(block)));
  return ranges;
}

function rendered(tex: string, display: boolean, whole: boolean, openLen: number) {
  return (view: EditorView, getPos: () => number | undefined) => {
    const el = document.createElement('span');
    el.className = whole ? 'math-render math-whole' : 'math-render';
    el.contentEditable = 'false';
    el.innerHTML = renderTex(tex, display);
    // A click puts the caret just inside the opening delimiter, which reveals the source for editing.
    el.addEventListener('mousedown', (e) => {
      const pos = getPos();
      if (pos === undefined || e.button !== 0) return;
      e.preventDefault();
      view.dispatch(view.state.tr.setSelection(TextSelection.create(view.state.doc, pos + openLen)));
      view.focus();
    });
    return el;
  };
}

function decorations(state: EditorState): MathPluginState {
  const { anchor, head } = state.selection;
  const touches = (from: number, to: number) => (anchor >= from && anchor <= to) || (head >= from && head <= to);
  const decos: Decoration[] = [];
  const ranges: { from: number; to: number }[] = [];
  state.doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    if (PLAIN.has(node.type.name)) return false;
    const start = pos + 1;
    for (const m of mathIn(node)) {
      const from = start + m.from;
      const to = start + m.to;
      ranges.push({ from, to });
      if (touches(from, to)) continue;
      const whole = m.display && m.from === 0 && m.to === node.content.size;
      const openLen = node.textBetween(m.from, m.from + 2) === '$$' || node.textBetween(m.from, m.from + 1) === '\\' ? 2 : 1;
      decos.push(
        Decoration.widget(from, rendered(m.tex, m.display, whole, openLen), {
          side: -1,
          ignoreSelection: true,
          key: `${isKatexReady() ? 'K' : 'L'}${m.display ? 'D' : 'I'}${whole ? 'W' : ''}${openLen}${m.tex}`,
        }),
        Decoration.inline(from, to, { class: 'math-source' }),
      );
    }
    return false;
  });
  if (ranges.length && !isKatexReady()) void ensureKatex();
  return { deco: ranges.length ? DecorationSet.create(state.doc, decos) : DecorationSet.empty, ranges };
}

function anyTouched(ranges: { from: number; to: number }[], anchor: number, head: number): boolean {
  for (const r of ranges) {
    if ((anchor >= r.from && anchor <= r.to) || (head >= r.from && head <= r.to)) return true;
  }
  return false;
}

/**
 * Live preview of `$…$` and `$$…$$` math. The note keeps the LaTeX source as plain text; a formula renders while the
 * caret is elsewhere and shows its source as soon as the caret touches it.
 */
export const MathPreview = Extension.create({
  name: 'mathPreview',
  addProseMirrorPlugins() {
    return [
      new Plugin<MathPluginState>({
        key: mathKey,
        state: {
          init: (_, state) => decorations(state),
          apply(tr, old, prev, next) {
            if (tr.getMeta(mathKey)) return decorations(next);
            if (tr.docChanged) return decorations(next);
            if (!tr.selectionSet || old.ranges.length === 0) return old;
            const prevTouched = anyTouched(old.ranges, prev.selection.anchor, prev.selection.head);
            const nextTouched = anyTouched(old.ranges, next.selection.anchor, next.selection.head);
            return prevTouched || nextTouched ? decorations(next) : old;
          },
        },
        view(editorView) {
          const off = onKatexReady(() => {
            if (!editorView.isDestroyed && mathKey.getState(editorView.state)?.ranges.length) {
              editorView.dispatch(editorView.state.tr.setMeta(mathKey, true));
            }
          });
          return { destroy: off };
        },
        props: { decorations: (state) => mathKey.getState(state)?.deco ?? DecorationSet.empty },
      }),
    ];
  },
});
