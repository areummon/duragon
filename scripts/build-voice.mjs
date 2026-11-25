// The app starts the voice helper from beside itself, but doesn't depend on it, so Cargo won't build it for the app.
// Tauri runs this before each dev run and build.
import { spawnSync } from 'node:child_process';

// Tauri sets the first for its every run, and the second only for a debug one.
const { TAURI_ENV_PLATFORM, TAURI_ENV_DEBUG } = process.env;
const release = TAURI_ENV_PLATFORM && TAURI_ENV_DEBUG !== 'true' ? ['--release'] : [];
const { status, error } = spawnSync('cargo', ['build', '-p', 'duragon-voice', '--bin', 'duragon-voice', ...release], { stdio: 'inherit' });
if (error) console.error(`could not run cargo: ${error.message}`);
process.exit(status ?? 1);
