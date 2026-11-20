import { Extension } from '@tiptap/core';
import type { Node as PMNode } from '@tiptap/pm/model';
import { type EditorState, Plugin, PluginKey, TextSelection } from '@tiptap/pm/state';
import { Decoration, DecorationSet, type EditorView } from '@tiptap/pm/view';
import { findMath, type MathRange, renderTex } from '../rich';

/** The title names the note in the sidebar as plain text, and code is code; neither renders math. */
const PLAIN = new Set(['title', 'codeBlock']);
const OPAQUE = '\uFFFC';
const mathKey = new PluginKey<DecorationSet>('duragon-math');

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

function decorations(state: EditorState): DecorationSet {
  const { anchor, head } = state.selection;
  const touches = (from: number, to: number) => (anchor >= from && anchor <= to) || (head >= from && head <= to);
  const decos: Decoration[] = [];
  state.doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    if (PLAIN.has(node.type.name)) return false;
    const start = pos + 1;
    for (const m of mathIn(node)) {
      const from = start + m.from;
      const to = start + m.to;
      if (touches(from, to)) continue;
      const whole = m.display && m.from === 0 && m.to === node.content.size;
      const openLen = node.textBetween(m.from, m.from + 2) === '$$' || node.textBetween(m.from, m.from + 1) === '\\' ? 2 : 1;
      decos.push(
        Decoration.widget(from, rendered(m.tex, m.display, whole, openLen), {
          side: -1,
          ignoreSelection: true,
          key: `${m.display ? 'D' : 'I'}${whole ? 'W' : ''}${openLen}${m.tex}`,
        }),
        Decoration.inline(from, to, { class: 'math-source' }),
      );
    }
    return false;
  });
  return DecorationSet.create(state.doc, decos);
}

/**
 * Live preview of `$…$` and `$$…$$` math. The note keeps the LaTeX source as plain text; a formula renders while the
 * caret is elsewhere and shows its source as soon as the caret touches it.
 */
export const MathPreview = Extension.create({
  name: 'mathPreview',
  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key: mathKey,
        state: {
          init: (_, state) => decorations(state),
          apply: (tr, old, _prev, next) => (tr.docChanged || tr.selectionSet ? decorations(next) : old),
        },
        props: { decorations: (state) => mathKey.getState(state) },
      }),
    ];
  },
});
