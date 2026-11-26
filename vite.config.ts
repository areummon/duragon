import { defineConfig, type Plugin } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import tauri from './src-tauri/tauri.conf.json' with { type: 'json' };

// Tauri sends its policy only with the built app. Sending it here too keeps `cargo tauri dev` as closed, and shows in
// dev what the policy would break. Hot reload needs its socket.
const policy: Record<string, string> = tauri.app.security.csp;
const csp = Object.entries(policy)
  .map(([name, sources]) => `${name} ${sources}${name === 'connect-src' ? " 'self' ws://localhost:*" : ''}`)
  .join('; ');

/** WebKitGTK / modern WebViews always support woff2; drop legacy woff/ttf fallbacks from KaTeX CSS (~876 KB saved). */
function katexWoff2Only(): Plugin {
  return {
    name: 'katex-woff2-only',
    enforce: 'pre',
    transform(code, id) {
      if (!id.includes('katex.min.css')) return null;
      return {
        code: code.replace(/,url\(fonts\/[^)]+\.woff\) format\("woff"\),url\(fonts\/[^)]+\.ttf\) format\("truetype"\)/g, ''),
        map: null,
      };
    },
  };
}

export default defineConfig({
  plugins: [katexWoff2Only(), svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    headers: { 'Content-Security-Policy': csp },
    // Transforms the whole app while cargo builds, so the window's first load is not the slow cold one.
    warmup: { clientFiles: ['./src/main.ts', './src/lib/components/customize/CustomizeDialog.svelte'] },
    // direnv's flake-inputs link into the Nix store; following them made every start crawl ~94k files.
    watch: { ignored: ['**/src-tauri/**', '**/target/**', '**/.direnv/**'] },
  },
});
