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
  // Restarted by rewinding rather than by re-adding a class, which would force a layout on every keystroke.
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const period = parseFloat(getComputedStyle(host).getPropertyValue('--dur-caret')) || 1060;
  const blink = still
    ? null
    : bar.animate([{ opacity: 1 }, { opacity: 1, offset: 0.5 }, { opacity: 0, offset: 0.5 }, { opacity: 0 }], { duration: period, iterations: Infinity });
  let height = '';

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
    const h = `${at.bottom - at.top}px`;
    if (h !== height) bar.style.height = height = h;
    // Solid while the writer types or moves, blinking once they pause, as the native caret does.
    if (blink) blink.currentTime = 0;
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
      blink?.cancel();
      bar.remove();
    },
  };
}
