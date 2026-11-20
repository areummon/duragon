const mac = /Mac|iPhone|iPad/.test(navigator.platform);

/** Shortcut label for the current platform: `kbd('K')` is "⌘K" on macOS and "Ctrl K" elsewhere. */
export function kbd(key: string, shift = false): string {
  if (mac) return `⌘${shift ? '⇧' : ''}${key}`;
  return `Ctrl ${shift ? 'Shift ' : ''}${key}`;
}

export const isMod = (e: KeyboardEvent) => (mac ? e.metaKey : e.ctrlKey);
