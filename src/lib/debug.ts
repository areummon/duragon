declare global {
  interface Window {
    __duragon?: Record<string, unknown>;
  }
}

/** Exposes reactive state on `window.__duragon` in dev builds without clobbering existing keys. */
export function exposeDebug(key: string, value: unknown): void {
  if (!import.meta.env.DEV || typeof window === 'undefined') return;
  window.__duragon ??= {};
  window.__duragon[key] = value;
}
