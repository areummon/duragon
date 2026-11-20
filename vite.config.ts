import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Transforms the whole app while cargo builds, so the window's first load is not the slow cold one.
    warmup: { clientFiles: ['./src/main.ts'] },
    // direnv's flake-inputs link into the Nix store; following them made every start crawl ~94k files.
    watch: { ignored: ['**/src-tauri/**', '**/target/**', '**/.direnv/**'] },
  },
});
