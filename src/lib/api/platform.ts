export interface FileFilter {
  name: string;
  extensions: string[];
}

export interface SaveFileOptions {
  defaultPath: string;
  filters: FileFilter[];
}

export interface OpenFileOptions {
  filters?: FileFilter[];
  directory?: boolean;
  mockFallback: string;
}

export const isTauri = () => '__TAURI_INTERNALS__' in window;

export async function pickSavePath(options: SaveFileOptions): Promise<string | null> {
  if (!isTauri()) return null;
  const { save } = await import('@tauri-apps/plugin-dialog');
  const picked = await save(options);
  return typeof picked === 'string' ? picked : null;
}

export async function pickOpenPath(options: OpenFileOptions): Promise<string | null> {
  if (!isTauri()) return options.mockFallback;
  const { open } = await import('@tauri-apps/plugin-dialog');
  const picked = await open({
    multiple: false,
    directory: options.directory,
    filters: options.filters,
  });
  return typeof picked === 'string' ? picked : null;
}

export async function openExternalUrl(url: string): Promise<void> {
  if (!isTauri()) {
    window.open(url, '_blank', 'noopener,noreferrer');
    return;
  }
  try {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  } catch {
    window.open(url, '_blank', 'noopener,noreferrer');
  }
}
