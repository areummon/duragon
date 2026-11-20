import { Extension } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';

/**
 * Draws the caret instead of WebKit. WebKit stretches its own caret on a wrapped line up into the leading above,
 * so past the first line of a paragraph it stood half again as tall. This one is always the height of the text.
 */
export const Caret = Extension.create({
  name: 'caret',
  addProseMirrorPlugins() {
    return [new Plugin({ view: (view) => caretView(view) })];
  },
});

function caretView(view: EditorView) {
  const host = view.dom.parentElement!;
  const bar = document.createElement('div');
  bar.className = 'caret';
  bar.setAttribute('aria-hidden', 'true');
  host.append(bar);
  view.dom.classList.add('drawn-caret');

  const place = () => {
    const { selection } = view.state;
    if (!view.hasFocus() || !selection.empty || !view.editable) {
      bar.hidden = true;
      return;
    }
    const at = view.coordsAtPos(selection.head);
    const origin = host.getBoundingClientRect();
    bar.hidden = false;
    bar.style.transform = `translate(${at.left - origin.left}px, ${at.top - origin.top}px)`;
    bar.style.height = `${at.bottom - at.top}px`;
    // Solid while the writer types or moves, blinking once they pause, as the native caret does.
    bar.classList.remove('blink');
    void bar.offsetWidth;
    bar.classList.add('blink');
  };

  const resized = new ResizeObserver(place);
  resized.observe(view.dom);
  view.dom.addEventListener('focus', place);
  view.dom.addEventListener('blur', place);
  place();

  return {
    update: place,
    destroy() {
      resized.disconnect();
      view.dom.removeEventListener('focus', place);
      view.dom.removeEventListener('blur', place);
      view.dom.classList.remove('drawn-caret');
      bar.remove();
    },
  };
}
