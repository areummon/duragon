import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tauri from './src-tauri/tauri.conf.json' with { type: 'json' };

// Tauri sends its policy only with the built app. Sending it here too keeps `cargo tauri dev` as closed, and shows in
// dev what the policy would break. Hot reload needs its socket.
const policy: Record<string, string> = tauri.app.security.csp;
const csp = Object.entries(policy)
  .map(([name, sources]) => `${name} ${sources}${name === 'connect-src' ? " 'self' ws://localhost:*" : ''}`)
  .join('; ');

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    headers: { 'Content-Security-Policy': csp },
    // Transforms the whole app while cargo builds, so the window's first load is not the slow cold one.
    warmup: { clientFiles: ['./src/main.ts'] },
    // direnv's flake-inputs link into the Nix store; following them made every start crawl ~94k files.
    watch: { ignored: ['**/src-tauri/**', '**/target/**', '**/.direnv/**'] },
  },
});
