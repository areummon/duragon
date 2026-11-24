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
- **Note header row.** The toolbar `Note settings`. It holds the mind picker (button named after the mind, e.g. `Duragon`), `Tell Duragon`, the mode picker (`Thinks with you`), the switches `Voice` and `Web`, and `Export`. Once the note has a brief, the `Tell Duragon` button moves below the row and shows the brief.
- **Editor.** The textbox is `Note`, and the title is the level-1 heading.
- **Side chat.** The input is `Ask about this note`, plus the buttons `Send`, `Copy`, `Clear`, `Pin open while you write` and `Close`.
- **Sidebar.** The navigation is `Notes`, with `Find or start a note Ctrl K`, `New note`, `New folder`, `Trash`, `Settings`, one button per note title and `Note options` per row. Entries in popover menus are `menuitem` (or `menuitemradio`, `menuitemcheckbox`), not `button`.
- **Customize.** Press `Control+,`. You get the navigation `Settings sections` (`Customize Duragon`, `Minds`, `About you`, `Voice models`, `Help`), the searchbox `Search settings` and `Close`.
- **Command palette.** Press `Control+k`. It is the dialog `Find a note`, with the combobox `Find a note, or name a new one` and the listbox `Notes`.
- **Editor menus.** Selected words show the toolbar `Selection`. A `/` shows the listbox `Commands`. A code block has the select `Code language` and the button `Copy code`.

`find … --name` matches a case-insensitive substring. Add `--exact` when one name contains another, such as `Copy` and `Copy code`, or `Close` and `Close notes`.

Useful patterns:

```sh
agent-browser find role button click --name "Chat"
agent-browser fill '[aria-label="Ask about this note"]' "What is the weakest claim here?"
agent-browser press Enter
agent-browser wait --text "write one sentence for each job"
agent-browser wait 3000        # replies ink in word by word; let them finish before a screenshot
```

Read backend state through the mock instance the page uses:

```sh
agent-browser eval '(async () => { const b = await (await import("/src/lib/api/backend.ts")).getBackend(); const n = (await b.listNotes()).find((x) => x.title === "Welcome to Duragon"); return (await b.listChat(n.id)).length; })()'
```

The import shares the page's module only while no file under `src/` has changed since `launch.sh` started Vite. After an edit, Vite serves the changed module as `…?t=<stamp>`, so the bare import loads a second copy with its own state. Run `cleanup.sh` and `launch.sh` again before you read the store.

Front-end state is on `window.__duragon` in dev builds, always as the page's own instance: `app` (open panels, `note`, `sidebar`, `mindEditor`), `notes`, `threads`, `settings` and `voice`. For example, `agent-browser eval '__duragon.app.customizeOpen'`.

Use both only to observe. Drive the change itself through the UI.

Headless Chromium denies `agent-browser clipboard read`. To prove a copy, capture what the page writes before you press the button:

```sh
agent-browser eval 'navigator.clipboard.writeText = async (t) => { window.__copied = t; }; "stubbed"'
agent-browser find role button click --name "Copy" --exact
agent-browser eval 'window.__copied'
```

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
- **Rendering engine.** `agent-browser` is Chromium. The app runs on WebKitGTK. For font, caret or animation fidelity, say the evidence came from Chromium, or drive WebKit's MiniBrowser through `nix develop -c WebKitWebDriver --port=4455` against this same Vite. MiniBrowser opens at an unpredictable size, so set the window with `POST /session/<id>/window/rect` to `{ "width": 1280, "height": 860 }` first. Below 1258 px, answers turn into `?` marks. Always delete the session at the end. The driver holds one session, so a script that dies before `DELETE /session/<id>` blocks the next run until you restart the driver.
- **Content security policy.** It lives in `src-tauri/tauri.conf.json`, and Vite sends the same one, so a blocked load shows here too. Listen for `securitypolicyviolation`, or read `agent-browser console`. Only the built app proves the policy Tauri sends.
- **Hover.** Headless Chromium reports `hover: none`, so `(hover: hover)` styles and hover tooltips won't fire. To prove a hover path, run a headed `agent-browser` inside a private Xvfb display (`nix shell nixpkgs#xvfb-run`).
- **IPv6 only.** Vite listens on `localhost` (`::1`). `127.0.0.1:5199` refuses connections.
- **Fresh mock on reload.** Reloading the page resets all mock data. Persistence across restarts can only be proven in the Rust tests (`tests/db.rs` reopens a database).
- **Blank page after a restart.** Once, right after `cleanup.sh` and `launch.sh`, the session sat on `about:blank` and evals said `__duragon is not defined`. Run `agent-browser open` again before you read anything. It did not happen in later restarts.
- **Panels stay mounted.** The closed side chat and sidebar stay in the DOM as `inert`. To check whether one is open, use `snapshot -i` or `__duragon.app`, not `querySelector`.
