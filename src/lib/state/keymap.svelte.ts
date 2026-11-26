import { exposeDebug } from '../debug';
import {
  KEYBOARD_ACTIONS,
  eventBinding,
  formatBinding,
  isReservedBinding,
  type KeyAction,
} from '../keys';

const STORAGE_KEY = 'duragon:keymap';

const DEFAULTS = new Map<KeyAction, string>(KEYBOARD_ACTIONS.map((a) => [a.id, a.binding]));
const ACTION_IDS = new Set<string>(KEYBOARD_ACTIONS.map((a) => a.id));

function isKeyAction(key: string): key is KeyAction {
  return ACTION_IDS.has(key);
}

function loadOverrides(): Partial<Record<KeyAction, string>> {
  if (typeof window === 'undefined') return {};
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object') return {};
    const out: Partial<Record<KeyAction, string>> = {};
    for (const [k, v] of Object.entries(parsed)) {
      if (!isKeyAction(k)) continue;
      const def = DEFAULTS.get(k);
      if (!def || typeof v !== 'string' || !v || isReservedBinding(v) || v === def) continue;
      out[k] = v;
    }
    return out;
  } catch {
    return {};
  }
}

function saveOverrides(overrides: Partial<Record<KeyAction, string>>): void {
  if (typeof window === 'undefined') return;
  try {
    if (Object.keys(overrides).length > 0) {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(overrides));
    } else {
      window.localStorage.removeItem(STORAGE_KEY);
    }
  } catch {
    // Storage unavailable; overrides still apply for this session.
  }
}

export const keymap = $state<{ overrides: Partial<Record<KeyAction, string>> }>({
  overrides: loadOverrides(),
});

export function bindingFor(id: KeyAction): string {
  return keymap.overrides[id] ?? DEFAULTS.get(id) ?? '';
}

export function bindingLabel(id: KeyAction): string {
  return formatBinding(bindingFor(id));
}

export function matchesAction(e: KeyboardEvent, id: KeyAction): boolean {
  const target = bindingFor(id);
  return target !== '' && eventBinding(e) === target;
}

export function setBinding(id: KeyAction, binding: string): { ok: true } | { ok: false; error: string } {
  if (isReservedBinding(binding)) {
    return { ok: false, error: 'Use a modifier chord (Ctrl/⌘ or Alt) or a function key.' };
  }
  const conflict = KEYBOARD_ACTIONS.find((a) => a.id !== id && bindingFor(a.id) === binding);
  if (conflict) {
    return { ok: false, error: `Already used by “${conflict.label}”.` };
  }
  const def = DEFAULTS.get(id);
  if (binding === def) {
    delete keymap.overrides[id];
  } else {
    keymap.overrides[id] = binding;
  }
  saveOverrides($state.snapshot(keymap.overrides));
  return { ok: true };
}

export function resetBinding(id: KeyAction): void {
  if (keymap.overrides[id] === undefined) return;
  delete keymap.overrides[id];
  saveOverrides($state.snapshot(keymap.overrides));
}

export function resetAllBindings(): void {
  if (Object.keys(keymap.overrides).length === 0) return;
  keymap.overrides = {};
  saveOverrides({});
}

export function isOverridden(id: KeyAction): boolean {
  return keymap.overrides[id] !== undefined;
}

export function hasAnyOverride(): boolean {
  return Object.keys(keymap.overrides).length > 0;
}

exposeDebug('keymap', keymap);
