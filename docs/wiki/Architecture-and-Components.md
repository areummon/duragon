# Architecture & Components

Duragon is a Tauri 2 desktop application split into a Rust core workspace and a Svelte 5 + TipTap frontend.

## Repository Layout

```text
duragon/
├── crates/
│   ├── core/                # Core domain, SQLite DB, AI runners, web search/fetch, export/import
│   │   ├── src/
│   │   │   ├── ai/          # runner.rs, http.rs, local.rs, acp.rs, codex.rs, tools.rs,
│   │   │   │                # search.rs, web_fetch.rs, notes_mcp.rs, tasks.rs, prompts.rs,
│   │   │   │                # veil.rs, quiet.rs, budget.rs, svg.rs
│   │   │   ├── db/          # mod.rs (migrations), notes.rs, revisions.rs, threads.rs,
│   │   │   │                # folders.rs, bundle.rs, misc.rs
│   │   │   ├── export/      # markdown.rs, docx.rs, import.rs, bulk.rs
│   │   │   ├── credentials.rs
│   │   │   ├── doc.rs
│   │   │   ├── model.rs
│   │   │   └── trash.rs
│   │   └── tests/           # 240+ offline integration tests
│   └── voice/               # Offline STT (Parakeet/Whisper) & TTS (Kokoro) helper
├── src-tauri/               # Tauri 2 shell, IPC commands (commands.rs), speechd, --notes-mcp entrypoint
├── src/
│   ├── lib/
│   │   ├── api/             # backend.ts, tauri.ts, mock.ts + mock/{fixtures,ai,voice}.ts,
│   │   │                    # platform.ts, defaults.ts, search.ts, providers.ts, types.ts
│   │   ├── editor/          # create.ts, Editor.svelte, vim.svelte.ts, vim-motions.ts,
│   │   │                    # find.svelte.ts, inline-menu.svelte.ts, InlineMenuPopup.svelte,
│   │   │                    # slash.svelte.ts, notelink.svelte.ts, riddle.svelte.ts, plain.ts
│   │   ├── components/      # Page, NoteHeaderRow, Sidebar, Margin, Thread, ChatPanel,
│   │   │                    # FindBar, HistoryDialog, TrashDialog, Modal, Popover, Drawing,
│   │   │                    # customize/ (CustomizeDialog + 8 focused settings pages)
│   │   └── state/           # Svelte 5 rune stores (app, notes, threads, chat, settings,
│   │                        # keymap, layout, voice, trash, saves)
│   └── styles/              # tokens.css, editor.css, ui.css, motion.css, riddle.css
└── docs/
    ├── demos/               # README demonstration GIFs
    ├── wiki/                # Documentation wiki
    └── architecture.md      # Original design notes
```

## SQLite Schema & Migrations

All database migrations live in `crates/core/src/db/mod.rs` using SQLite `PRAGMA user_version` with `PRAGMA foreign_keys = ON`:

- `notes` — note metadata, ProseMirror JSON (`doc`), plain text (`plain`), folder, Mind, mode, Voice/Web flags, and brief.
- `folders` — nested folder hierarchy (`parent_id`).
- `threads` & `messages` — margin threads, inline replies, chat messages, SVG sketches, web sources, and math symbols.
- `chat_summaries` — rolling chat compression watermarks.
- `riddlers` — built-in and custom Minds.
- `note_revisions` — throttled per-note snapshots (`(note_id, created_at DESC)` index, 20-item cap, transactional restore).
- `note_links` — indexed `(from_note_id, to_note_id)` join table powering fast backlinks.
- `kv` — persisted app settings.

## Key Invariants

- **Non-blocking IPC:** SQLite, export/import, and trash I/O run on Tokio's blocking pool (`db_blocking` in `src-tauri/src/commands.rs`).
- **Browser-testable frontend:** `src/lib/api/backend.ts` switches cleanly between `tauri.ts` (desktop) and `mock.ts` (browser `npm run dev`), so the entire UI can be driven and verified without Tauri or live LLM calls.
- **Security boundaries:** SVG output is allowlist-sanitized before storage; `web_fetch` enforces DNS-pinned public-IP allowlists and `.no_proxy()`; bulk export rejects symlinks and avoids overwrites with `create_new`; `Veil` masks secrets and PII before outbound requests; CLI runners run with unrelated API keys stripped (`quiet.rs`).
