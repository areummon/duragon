import { current } from './editor/current';
import { focusLost, topDialog } from './focus';

interface Layer {
  close: () => void;
  /** What had focus when the layer opened. It gets focus back when closing leaves focus nowhere. */
  from: Element | null;
  /** Stands in for `from` when opening the layer removed it. */
  instead?: () => HTMLElement | null | undefined;
}

/**
 * Everything Escape can close, oldest first: open menus, panels and dialogs. One press closes the newest, so a menu
 * goes before the dialog it sits in. A field that uses Escape itself, such as renaming a note, stops the event first.
 */
const layers: Layer[] = [];

let lastFocused: Element | null = null;
window.addEventListener('focusin', (e) => (lastFocused = e.target as Element));

const usable = (el: Element | null | undefined): el is HTMLElement =>
  el instanceof HTMLElement && el !== document.body && el.isConnected && !el.closest('[inert]');

/** To what had focus before the layer opened, else to the note. */
function giveBack(layer: Layer) {
  if (!focusLost()) return;
  const to = [layer.from, layer.instead?.(), topDialog() ?? current.editor?.view.dom].find(usable);
  to?.focus({ preventScroll: true });
}

/** Lets Escape close something while it is open; returns the function that takes it off again. */
export function closeOnEscape(close: () => void, instead?: Layer['instead']): () => void {
  const layer = { close, from: document.activeElement, instead };
  layers.push(layer);
  return () => {
    const i = layers.indexOf(layer);
    if (i < 0) return;
    layers.splice(i, 1);
    // Closed some other way. Focus that went with it comes back; focus the writer took elsewhere stays.
    if (!usable(lastFocused)) giveBack(layer);
  };
}

/** Closes the newest layer, if any. */
export function closeTop() {
  const layer = layers.pop();
  if (!layer) return;
  layer.close();
  requestAnimationFrame(() => giveBack(layer));
}

/** Dismisses an open popup on outside click/focus or Escape; returns cleanup. */
export function dismissOnOutside(
  root: () => HTMLElement | null | undefined,
  close: () => void,
  focusBack?: Layer['instead'],
): () => void {
  const away = (e: Event) => {
    const el = root();
    if (!el || !el.contains(e.target as Node)) close();
  };
  window.addEventListener('pointerdown', away, true);
  window.addEventListener('focusin', away);
  const off = closeOnEscape(close, focusBack);
  return () => {
    window.removeEventListener('pointerdown', away, true);
    window.removeEventListener('focusin', away);
    off();
  };
}
