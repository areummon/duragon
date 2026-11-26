const mac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);

/** Shortcut label for the current platform: `kbd('K')` is "⌘K" on macOS and "Ctrl K" elsewhere. */
export function kbd(key: string, shift = false): string {
  if (mac) return `⌘${shift ? '⇧' : ''}${key}`;
  return `Ctrl ${shift ? 'Shift ' : ''}${key}`;
}

export const isMod = (e: KeyboardEvent) => e.metaKey || e.ctrlKey;

/** By its place on the keyboard: WebKitGTK reports Shift+Tab's `key` as "Unidentified". */
export const isTab = (e: KeyboardEvent) => e.code === 'Tab';

export type KeyAction =
  | 'find-note'
  | 'new-note'
  | 'toggle-chat'
  | 'open-customize'
  | 'find-in-note'
  | 'copy-note'
  | 'export-note'
  | 'just-my-words'
  | 'riddle-selection';

export type KeyScope = 'navigation' | 'note';

export interface KeyActionDef {
  id: KeyAction;
  label: string;
  scope: KeyScope;
  binding: string;
}

export const KEYBOARD_ACTIONS: readonly KeyActionDef[] = [
  { id: 'find-note', label: 'Find or start a note', scope: 'navigation', binding: 'mod+k' },
  { id: 'new-note', label: 'New note', scope: 'navigation', binding: 'mod+n' },
  { id: 'toggle-chat', label: 'Open or close chat', scope: 'navigation', binding: 'mod+j' },
  { id: 'open-customize', label: 'Open settings', scope: 'navigation', binding: 'mod+,' },
  { id: 'find-in-note', label: 'Find in this note', scope: 'note', binding: 'mod+f' },
  { id: 'copy-note', label: 'Copy this note', scope: 'note', binding: 'mod+shift+c' },
  { id: 'export-note', label: 'Export this note', scope: 'note', binding: 'mod+shift+e' },
  { id: 'just-my-words', label: 'Just my words', scope: 'note', binding: 'mod+shift+.' },
  { id: 'riddle-selection', label: 'Ask Mind about selection', scope: 'note', binding: 'mod+enter' },
];

function normalizeKey(e: KeyboardEvent, hasModOrAlt: boolean): string {
  if (e.code === 'Tab' || e.key === 'Tab') return 'tab';
  if (e.key === ' ' || e.code === 'Space') return 'space';
  if (e.key === 'Enter' || e.code === 'Enter') return 'enter';
  if (e.key === 'Escape') return 'escape';

  if (hasModOrAlt) {
    // Shift+Period reports '>' on US layouts; keep the physical punctuation key when a modifier is held.
    if (e.code === 'Period' && (e.shiftKey || e.key > '\x7f')) return '.';
    if (e.code === 'Comma' && (e.shiftKey || e.key > '\x7f')) return ',';
    if (e.key.length === 1 && e.key > '\x7f') {
      const letter = /^Key([A-Z])$/.exec(e.code);
      if (letter?.[1]) return letter[1].toLowerCase();
    }
  }

  return e.key.toLowerCase();
}

/**
 * Normalize a KeyboardEvent to a canonical cross-platform binding string.
 * Order is fixed: `mod`, `alt`, `shift`, then the key.
 * `mod` matches both Meta (⌘) and Ctrl.
 */
export function eventBinding(e: KeyboardEvent): string {
  const mod = isMod(e);
  const hasModOrAlt = mod || e.altKey;
  const key = normalizeKey(e, hasModOrAlt);

  if (!hasModOrAlt && e.key.length === 1 && key !== 'space') return key;

  const mods: string[] = [];
  if (mod) mods.push('mod');
  if (e.altKey) mods.push('alt');
  if (e.shiftKey) mods.push('shift');
  return [...mods, key].join('+');
}

/**
 * True for keys that cannot be bound to app shortcuts: Escape, Tab / Shift+Tab,
 * and bare keys without `mod`, `alt`, or a function key (`f1`..`f12`).
 */
export function isReservedBinding(binding: string): boolean {
  if (!binding) return true;
  if (binding === 'escape' || binding === 'tab' || binding === 'shift+tab') return true;
  if (binding.startsWith('mod+') || binding.startsWith('alt+')) return false;
  if (/^f([1-9]|1[0-2])$/.test(binding)) return false;
  return true;
}

const KEY_GLYPH: Record<string, string> = {
  arrowup: '↑',
  arrowdown: '↓',
  arrowleft: '←',
  arrowright: '→',
  enter: '↵',
  tab: '⇥',
  escape: 'Esc',
  space: 'Space',
  backspace: '⌫',
  delete: 'Del',
};

const MOD_LABEL: Record<string, [mac: string, other: string]> = {
  mod: ['⌘', 'Ctrl'],
  alt: ['⌥', 'Alt'],
  shift: ['⇧', 'Shift'],
};

/** Format a normalized binding for display: `'mod+k'` -> `'⌘K'` on macOS or `'Ctrl K'` elsewhere. */
export function formatBinding(binding: string): string {
  if (!binding) return '';
  const parts = binding.split('+');
  let key = parts.pop() ?? '';
  if (key === '' && parts.length > 0 && parts[parts.length - 1] === '') {
    parts.pop();
    key = '+';
  }
  const keyGlyph = KEY_GLYPH[key] ?? key.toUpperCase();
  const tokens = parts.flatMap((part) => {
    const pair = MOD_LABEL[part];
    return pair ? [pair[mac ? 0 : 1]] : [];
  });
  return [...tokens, keyGlyph].join(mac ? '' : ' ');
}
