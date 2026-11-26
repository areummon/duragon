// The app starts the voice helper from beside itself, but doesn't depend on it, so Cargo won't build it for the app.
// Tauri runs this before each dev run and build, and copies the helper into src-tauri/binaries/ so bundles include it.
import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const { TAURI_ENV_PLATFORM, TAURI_ENV_DEBUG } = process.env;
const isRelease = Boolean(TAURI_ENV_PLATFORM && TAURI_ENV_DEBUG !== 'true');
const release = isRelease ? ['--release'] : [];

const build = spawnSync('cargo', ['build', '-p', 'duragon-voice', '--bin', 'duragon-voice', ...release], { stdio: 'inherit' });
if (build.error) console.error(`could not run cargo: ${build.error.message}`);
if ((build.status ?? 1) !== 0) process.exit(build.status ?? 1);

const rustc = spawnSync('rustc', ['-vV'], { encoding: 'utf8' });
const host = rustc.stdout?.match(/^host:\s*(\S+)/m)?.[1];
if (!host) {
  console.error('could not determine rustc host triple');
  process.exit(1);
}

const ext = host.includes('windows') ? '.exe' : '';
const src = resolve('target', isRelease ? 'release' : 'debug', `duragon-voice${ext}`);
const dir = resolve('src-tauri', 'binaries');
mkdirSync(dir, { recursive: true });
copyFileSync(src, resolve(dir, `duragon-voice-${host}${ext}`));
