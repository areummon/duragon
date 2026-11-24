interface Layer {
  close: () => void;
  /** What had focus when the layer opened. It gets focus back when closing leaves focus nowhere. */
  from: Element | null;
}

/**
 * Everything Escape can close, oldest first: open menus, panels and dialogs. One press closes the newest, so a menu
 * goes before the dialog it sits in. A field that uses Escape itself, such as renaming a note, stops the event first.
 */
const layers: Layer[] = [];

/** Lets Escape close something while it is open; returns the function that takes it off again. */
export function closeOnEscape(close: () => void): () => void {
  const layer = { close, from: document.activeElement };
  layers.push(layer);
  return () => {
    const i = layers.indexOf(layer);
    if (i >= 0) layers.splice(i, 1);
  };
}

/** Closes the newest layer, if any. */
export function closeTop() {
  const layer = layers.pop();
  if (!layer) return;
  layer.close();
  requestAnimationFrame(() => {
    const now = document.activeElement;
    if (now && now !== document.body && !now.closest('[inert]')) return;
    if (layer.from instanceof HTMLElement && layer.from.isConnected) layer.from.focus({ preventScroll: true });
  });
}
