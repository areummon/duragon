// The app starts the voice helper from beside itself, but doesn't depend on it, so Cargo won't build it for the app.
// Tauri runs this before each dev run and build, and copies the helper into src-tauri/binaries/ so bundles include it.
import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const { TAURI_ENV_PLATFORM, TAURI_ENV_DEBUG, TAURI_ENV_TARGET_TRIPLE } = process.env;
const isRelease = Boolean(TAURI_ENV_PLATFORM && TAURI_ENV_DEBUG !== 'true');
const release = isRelease ? ['--release'] : [];
const explicitTarget = TAURI_ENV_TARGET_TRIPLE?.trim() || '';
const targetArgs = explicitTarget ? ['--target', explicitTarget] : [];

const build = spawnSync('cargo', ['build', '-p', 'duragon-voice', '--bin', 'duragon-voice', ...targetArgs, ...release], {
  stdio: 'inherit',
});
if (build.error) console.error(`could not run cargo: ${build.error.message}`);
if ((build.status ?? 1) !== 0) process.exit(build.status ?? 1);

let triple = explicitTarget;
if (!triple) {
  const rustc = spawnSync('rustc', ['-vV'], { encoding: 'utf8' });
  triple = rustc.stdout?.match(/^host:\s*(\S+)/m)?.[1] ?? '';
  if (!triple) {
    console.error('could not determine rustc host triple');
    process.exit(1);
  }
}

const ext = triple.includes('windows') ? '.exe' : '';
const profile = isRelease ? 'release' : 'debug';
const src = explicitTarget
  ? resolve('target', explicitTarget, profile, `duragon-voice${ext}`)
  : resolve('target', profile, `duragon-voice${ext}`);
const dir = resolve('src-tauri', 'binaries');
mkdirSync(dir, { recursive: true });
copyFileSync(src, resolve(dir, `duragon-voice-${triple}${ext}`));
