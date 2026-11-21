---
name: verify
description: Drive Duragon (Tauri 2 + Svelte 5 notes app with AI "minds") to prove a change works. Use after any UI change to drive the real frontend in a browser against the in-memory mock backend, and after any Rust core change to run the fake-CLI tests and, when model behaviour matters, the real `claude` CLI smoke tests. Covers launch, doctor, drive, evidence and cleanup.
---

# Verify Duragon

Duragon has three surfaces. Pick by what the change touched.

| Change touched | Surface | How |
| --- | --- | --- |
| `src/**` (Svelte, CSS, TS) | Frontend on the mock backend | Vite on :5199 + `agent-browser` (this file) |
| `crates/core/**` (DB, prompts, AI tasks) | Rust core against a fake `claude` | `nix develop -c cargo test -p duragon-core` |
| Prompts or anything whose result depends on the model | Rust core against the real `claude` CLI | `#[ignore]` tests in `crates/core/tests/real_cli.rs` |
| `src-tauri/**` (commands, startup) | The built binary | `nix develop -c cargo build -p duragon`, plus the frontend run for the calling UI |

The full desktop app (Tauri window on WebKitGTK) is not driven here. See Gotchas.

Feature recipes live in `features/`. Read `features/README.md`, then the matching feature file.

## Launch

```sh
.claude/skills/verify/scripts/launch.sh
```

It starts Vite for the frontend on `http://localhost:5199` (override with `PORT=`; 1420 is refused) and writes its pid to `$VERIFY_DIR/vite.pid` (default `/tmp/duragon-verify`). It runs `npm ci` first if `node_modules` is missing. Ready is printed once `/src/main.ts` is served, normally within 3 seconds.

Outside Tauri, `src/lib/api/backend.ts` picks `MockBackend` (`src/lib/api/mock.ts`). Each page load starts from fresh seed data, with two notes, "Welcome to Duragon" (with one open margin thread) and "Scratch". The mock answers AI calls with canned text after short timers. A question containing "formula" or "code" gets a rich answer with math and code.

The page is usable when `.ProseMirror` exists:

```sh
export AGENT_BROWSER_SESSION=duragon-verify
agent-browser open http://localhost:5199/
agent-browser set viewport 1280 860
agent-browser wait .ProseMirror
```

## Doctor

```sh
.claude/skills/verify/scripts/doctor.sh
```

It is read-only. It checks that this run's Vite is alive, that it owns the port, and that the port serves the Duragon frontend. It also prints the git revision (and whether the tree is dirty) and whether the user's own `cargo tauri dev` is up on :1420. Run it first whenever anything looks off. It ends with `doctor: OK` or exits 1.

## Drive

Use `agent-browser` with `AGENT_BROWSER_SESSION=duragon-verify`, so this run's browser stays separate from any other. Prefer accessible names from `agent-browser snapshot -i` over CSS. These names are stable:

- **Top bar.** The buttons are `Open notes` (or `Close notes` while open) and `Chat`.
- **Note header row.** It holds the mind picker (button named after the mind, e.g. `Duragon`), `Tell Duragon`, the mode picker (`Thinks with you`), the switches `Voice` and `Web`, and `Export`.
- **Editor.** The textbox is `Note`, and the title is the level-1 heading.
- **Side chat.** The input is `Ask about this note`, plus the buttons `Send`, `Copy`, `Clear`, `Pin open while you write` and `Close`.
- **Sidebar.** The navigation is `Notes`, with `Find or start a note`, `New note`, `New folder`, `Settings`, one button per note title and `Note options` per row.
- **Customize.** Press `Control+,`. You get the navigation `Settings sections` (`Customize Duragon`, `Minds`, `About you`, `Voice models`, `Help`), the searchbox `Search settings` and `Close`.

Useful patterns:

```sh
agent-browser find role button click --name "Chat"
agent-browser fill '[aria-label="Ask about this note"]' "What is the weakest claim here?"
agent-browser press Enter
agent-browser wait --text "write one sentence for each job"
agent-browser wait 3000        # replies ink in word by word; let them finish before a screenshot
```

Read or change backend state the way the UI does, through the same mock instance the page uses. The page and the dynamic import share one module, so this is the real store and not a copy:

```sh
agent-browser eval '(async () => { const b = await (await import("/src/lib/api/backend.ts")).getBackend(); const n = (await b.listNotes()).find((x) => x.title === "Welcome to Duragon"); return (await b.listChat(n.id)).length; })()'
```

Use this only to observe side effects. Drive the change itself through the UI.

## Evidence

Save proofs under `$VERIFY_DIR/evidence/<feature>/` (default `/tmp/duragon-verify/evidence/`). Cleanup never deletes it.

- **Before and after.** Capture the action and the resulting state, not only the final screen. Take a screenshot before and after (`agent-browser screenshot <path>`) and an accessibility snapshot of the end state (`agent-browser snapshot -i > <path>.aria.txt`).
- **Side effects.** Prove them alongside what's visible, by reading the mock store with the eval above before and after the action. A UI that only looks cleared is not proof that the data is cleared.
- **Rust changes.** Paste the `test result:` lines. For a real-CLI test, paste the printed model output.
- **Mocks.** The mock backend stands in only for the Tauri IPC boundary. It does not prove Rust behaviour. A change that crosses IPC needs both the frontend run and the Rust tests.

## Rust core

```sh
nix develop -c cargo test -p duragon-core                     # fake `claude` scripts, no network
nix develop -c cargo build -p duragon                         # the Tauri binary, including --notes-mcp
nix develop -c cargo test -p duragon-core --test real_cli -- --ignored --nocapture <test name>
```

The real-CLI tests call the user's `claude` login and spend tokens. Run only the one that matches the change, on haiku. List them with `grep -n "async fn real_" crates/core/tests/real_cli.rs`.

## Cleanup

```sh
.claude/skills/verify/scripts/cleanup.sh
```

It closes this run's `agent-browser` session, kills only the process group in `vite.pid`, and confirms the port is closed. Evidence stays. Run it after every failed attempt too. Never `pkill -f`, and never kill by process name.

## Gotchas

- **Port 1420.** It belongs to the user's `cargo tauri dev`. The scripts refuse it. Never stop or restart the user's session.
- **No full app run.** Don't launch a `tauri-driver` copy of the app while the user's session runs. The built app's devUrl is fixed to :1420, and twice a parallel test copy coincided with the user's session dying. If a change can only be proven in the real Tauri window, check `ss -ltn | grep :1420` and ask the user to stop their session first.
- **Rendering engine.** `agent-browser` is Chromium. The app runs on WebKitGTK. For font, caret or animation fidelity, say the evidence came from Chromium, or drive WebKit's MiniBrowser through `nix develop -c WebKitWebDriver --port=4455` against this same Vite.
- **Hover.** Headless Chromium reports `hover: none`, so `(hover: hover)` styles and hover tooltips won't fire. To prove a hover path, run a headed `agent-browser` inside a private Xvfb display (`nix shell nixpkgs#xvfb-run`).
- **IPv6 only.** Vite listens on `localhost` (`::1`). `127.0.0.1:5199` refuses connections.
- **Fresh mock on reload.** Reloading the page resets all mock data. Persistence across restarts can only be proven in the Rust tests (`tests/db.rs` reopens a database).
