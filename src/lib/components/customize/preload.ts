import type { Component } from 'svelte';

let pending: Promise<{ default: Component }> | null = null;

export function loadCustomizeDialog(): Promise<{ default: Component }> {
  return (pending ??= import('./CustomizeDialog.svelte').catch((err) => {
    pending = null;
    throw err;
  }));
}

export function preloadCustomizeDialog(): void {
  void loadCustomizeDialog().catch(() => {});
}
