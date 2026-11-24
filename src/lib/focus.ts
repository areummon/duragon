import { tick } from 'svelte';
import { isTab } from './keys';

const FOCUSABLE = 'a[href], button, input, select, textarea, summary, [contenteditable="true"], [tabindex]';

/** The dialog on top, if one is open. A dialog on its way out is inert. */
export const topDialog = () => [...document.querySelectorAll<HTMLElement>('[aria-modal="true"]:not([inert])')].at(-1);

const canFocus = (el: HTMLElement) =>
  (el.tabIndex >= 0 || el.isContentEditable) && !el.matches(':disabled') && !el.closest('[inert]') && el.checkVisibility({ visibilityProperty: true });

const stops = (root: HTMLElement) => [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(canFocus);

/** Nothing usable has focus: it fell to the page, or sits in something that closed. */
export function focusLost() {
  const now = document.activeElement;
  return !now || now === document.body || !!now.closest('[inert]');
}

/** Tab goes round the open dialog, or round the window: from the last control to the first, and back with Shift. */
export function cycleTab(e: KeyboardEvent) {
  if (!isTab(e) || e.defaultPrevented || e.isComposing || e.ctrlKey || e.altKey || e.metaKey) return;
  const dialog = topDialog();
  const all = stops(dialog ?? document.body);
  const now = document.activeElement;
  const end = e.shiftKey ? all[0] : all.at(-1);
  // Focus on the dialog itself has nothing before it. Focus behind the dialog comes in.
  const outside = dialog && (!dialog.contains(now) || (now === dialog && e.shiftKey));
  if (!outside && now !== end) return;
  e.preventDefault();
  (e.shiftKey ? all.at(-1) : all[0])?.focus();
}

/**
 * Keys for an open menu. Opened from the keyboard, it takes focus. Arrows move through it, and Tab closes it and goes on
 * from what opened it.
 */
export function menuKeys(node: HTMLElement, onclose: () => void) {
  const items = () => [...node.querySelectorAll<HTMLElement>('[role^="menuitem"]')];
  const opener = document.activeElement;
  // After the menu's layer has noted what had focus.
  if (opener instanceof HTMLButtonElement && opener.matches(':focus-visible')) void tick().then(() => items()[0]?.focus());

  const key = (e: KeyboardEvent) => {
    const all = items();
    const i = all.indexOf(document.activeElement as HTMLElement);
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      const next = e.key === 'ArrowDown' ? i + 1 : Math.max(i, 0) - 1;
      all[(next + all.length) % all.length]?.focus();
    } else if (isTab(e) && !node.contains(document.activeElement)) all[0]?.focus();
    else {
      if (isTab(e)) {
        if (opener instanceof HTMLElement) opener.focus();
        onclose();
      }
      return;
    }
    e.preventDefault();
    e.stopPropagation();
  };
  // An item that swaps the menu's content takes its own focus with it.
  const swapped = (e: MouseEvent) => {
    if (e.detail === 0) requestAnimationFrame(() => node.isConnected && !node.inert && focusLost() && items()[0]?.focus());
  };
  window.addEventListener('keydown', key, true);
  node.addEventListener('click', swapped);
  return {
    destroy() {
      window.removeEventListener('keydown', key, true);
      node.removeEventListener('click', swapped);
    },
  };
}

/** A dialog takes focus as it opens, once its layer has noted what had it. */
export function takeFocus(node: HTMLElement) {
  void tick().then(() => node.focus({ preventScroll: true }));
}
