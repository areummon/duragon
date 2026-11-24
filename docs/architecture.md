# Duragon architecture

Duragon is a personal, open-source notebook in the style of Riddled (see `reference-riddled.md`). You write; Claude reads along and asks questions in the margin, answers when addressed, and sketches. It runs on the user's own Claude subscription by spawning the installed `claude` CLI.

Desktop first (Tauri 2). The core crate has no Tauri dependency so a web server can wrap it later.

## Layout

```
flake.nix                 dev shell: rust, cargo-tauri, node, webkitgtk_4_1, gtk3, libsoup_3, openssl, pkg-config
Cargo.toml                workspace: crates/core, src-tauri
crates/core/              duragon-core: storage + AI. No tauri imports.
  src/lib.rs
  src/model.rs            domain types (serde, shared with the frontend through types.ts)
  src/db/                 rusqlite, migrations, one module per table group; bundle.rs takes a whole note out and puts it back
  src/trash.rs            deleted notes: the .trash/ folder, its file format and its lock
  src/ai/                 claude runner, prompts, task orchestration
  src/export/             markdown + docx from ProseMirror JSON
src-tauri/                thin shell: commands.rs maps 1:1 onto core functions, emits events; system_trash.rs wraps the trash crate
src/                      Svelte 5 + Vite + TypeScript frontend
  lib/api/backend.ts      the Backend interface (the only way the UI talks to Rust)
  lib/api/tauri.ts        Backend over invoke/listen
  lib/api/mock.ts         in-memory Backend with canned AI replies, for browser dev and UI tests
  lib/state/              rune-based stores (notes, threads, settings, riddlers)
  lib/editor/             TipTap setup and extensions
  lib/components/         UI
  lib/motion/             animation helpers (ink words, blot, tether, drawing reveal)
  styles/                 tokens.css (themes), motion.css (keyframes), base.css
docs/
```

The frontend picks `mock` when `window.__TAURI_INTERNALS__` is absent, so `npm run dev` in a normal browser shows the whole UI with fake data. This is how UI work is verified with agent-browser.

## Domain model

All ids are UUID v4 strings. Times are unix milliseconds (i64 / number).

```ts
type ReadingMode = 'thinks' | 'asks';            // "Thinks with you" | "When you ask"
type Tone = 'plain' | 'warm' | 'blunt' | 'riddle';
type Frequency = 'rarely' | 'matters' | 'often';

interface NoteMeta { id; title; folderId: string | null; updatedAt; hasOpenQuestion: boolean }
interface Note extends NoteMeta {
  doc: unknown;               // TipTap/ProseMirror JSON. Source of truth for content.
  riddlerId: string;          // builtin Riddled = '00000000-0000-4000-8000-000000000001'
  mode: ReadingMode;
  voiceOn: boolean;
  webOn: boolean;
  brief: string;              // "Tell Riddled" text, '' when unset
  createdAt;
}
interface Folder { id; name; createdAt }

type ThreadKind = 'question' | 'reply';          // question: unprompted, margin. reply: addressed, inline under the line
type ThreadStatus = 'open' | 'resolved' | 'dismissed';
interface Thread { id; noteId; kind: ThreadKind; quote: string; status: ThreadStatus; pinned: boolean; createdAt; messages: Message[] }
type DrawingState = 'none' | 'pending' | 'done' | 'failed';
interface Message { id; noteId; threadId: string | null; author: 'you' | 'riddled'; text: string; drawing: DrawingState; svg: string | null;
  sources: Source[]; symbols: MathSymbol[]; trimmed: boolean; createdAt }
// trimmed: the note itself was cut to fit the payload budget (see "Payload budget"); cutting old chat messages or closed threads does not count
// chat panel messages are Message rows with threadId = null and a noteId

interface Riddler { id; name; description; howItThinks; tone: Tone; frequency: Frequency; builtin: boolean }

interface Settings {
  newNoteMode: ReadingMode;            // default 'thinks'
  readingPace: 'regular' | 'slow';     // debounce 900 ms | 1800 ms
  questionPlacement: 'both' | 'right' | 'marks';
  webInNewNotes: boolean;
  voiceInNewNotes: boolean;
  voiceName: string;
  speakingSpeed: number;               // 0.5..2, default 1
  appearance: 'system' | 'light' | 'dark';
  textSize: 'small' | 'default' | 'large';   // 15 / 16 / 18 px
  pageWidth: number | null;            // px, null = default 690
  lettering: 'own' | 'notes';          // Newsreader serif | same as notes
  inkColour: 'ink' | 'blue' | 'green' | 'red' | 'purple' | 'brown' | 'orange';
  justMyWords: boolean;                // hide all Riddled marks
  aboutYou: string;                    // <= 700 chars
  learnAboutYou: boolean;
  searchMyNotes: boolean;              // default false; side chat may search the other notes
  explainSymbols: boolean;             // default false; the Mind says what each symbol in its formulas stands for
  readModel: string;                   // default 'sonnet'  (read-along, answers, replies)
  chatModel: string;                   // default 'sonnet'
  drawModel: string;                   // default 'sonnet'
  sidebarOpen: boolean;
  deletedNotes: 'duragon' | 'system' | 'permanent';   // default 'duragon'; see "Trash"
}
```

## SQLite schema (crates/core/src/db/migrations)

```sql
notes(id TEXT PK, title TEXT, doc TEXT, plain TEXT, folder_id TEXT NULL, riddler_id TEXT, mode TEXT,
      voice_on INT, web_on INT, brief TEXT, created_at INT, updated_at INT)
notes_fts USING fts5(title, plain, content='notes', content_rowid='rowid')  + triggers
folders(id TEXT PK, name TEXT, created_at INT)
threads(id TEXT PK, note_id TEXT REFERENCES notes ON DELETE CASCADE, kind TEXT, quote TEXT, status TEXT, pinned INT, created_at INT)
messages(id TEXT PK, note_id TEXT, thread_id TEXT NULL REFERENCES threads ON DELETE CASCADE, author TEXT,
         text TEXT, drawing TEXT, svg TEXT NULL, sources TEXT DEFAULT '[]', symbols TEXT DEFAULT '[]', trimmed INT DEFAULT 0, created_at INT)
         -- sources: [{url, title}] the Mind used on the web; symbols: [{tex, meaning}] for its formulas;
         -- trimmed: 1 when the note itself was cut to fit the payload budget
riddlers(id TEXT PK, name, description, how_it_thinks, tone, frequency, builtin INT)
chat_summaries(note_id TEXT PK REFERENCES notes ON DELETE CASCADE, chat_summary TEXT DEFAULT '',
               chat_summarized_through TEXT NULL)   -- the side chat's summary and the id of the last message it covers
kv(key TEXT PK, value TEXT)          -- settings JSON under key 'settings'
```

Database file: `$XDG_DATA_HOME/duragon/duragon.db` (Tauri `app_data_dir`). `PRAGMA journal_mode=WAL; foreign_keys=ON`. Migrations run on open and are idempotent (`user_version`).

The `plain` column is derived from `doc` by the frontend on save (title + block text joined by newlines) and is what AI tasks and search read.

## Backend interface (Tauri commands; same names in backend.ts, camelCase args)

```
list_notes() -> NoteMeta[]                    newest first
search_notes(query) -> NoteMeta[]             FTS5, prefix match
get_note(id) -> Note
create_note(title?) -> Note                   applies settings defaults (mode, web, voice)
save_note(id, title, doc, plain) -> NoteMeta
update_note_meta(id, patch: {riddlerId?, mode?, voiceOn?, webOn?, brief?, folderId?}) -> Note
delete_note(id, mode: 'duragon'|'system'|'permanent') -> mode   where it went; 'duragon' when the system trash refused it
list_trash() -> {notes: {id, title, deletedAt}[], unreadable}   newest first
restore_note(id) -> NoteMeta                  already live: returned as it is
delete_trashed(ids)                           deletes these from the trash for good, passing over any that is live
list_folders() / create_folder(name) / rename_folder(id,name) / delete_folder(id)

list_threads(noteId) -> Thread[]              includes messages
set_thread_status(id, status) / set_thread_pinned(id, pinned)
thread_to_note(id) -> Note                    "Make a note"
list_chat(noteId) -> Message[]
get_chat_summary(noteId) -> string            '' until the chat has folded any messages
save_chat_summary(noteId, base, text) -> bool  the writer's edit of `base`; false when the summary moved on; the watermark stays
clear_chat(noteId)                            cancels an in-flight chat reply, deletes the side chat and its summary

get_settings() -> Settings / save_settings(Settings)
list_riddlers() / save_riddler(Riddler) / delete_riddler(id)

ai_read(noteId, target, nth)                  fire and forget; result arrives as events; nth = which occurrence of target's text, from 0
ai_answer(noteId, target, nth)                same nth as ai_read: the copy of the addressed line
ai_ask(noteId, target, question | null, nth)  selection actions "{mind} this" (null) and "Ask {mind}"; nth = which occurrence of the selection
ai_reply(threadId, text)
ai_chat(noteId, text)
ai_cancel_read(noteId)                        user typed again: kill in-flight read for the note

export_note(id, format: 'md'|'docx', path)    PDF goes through the webview's print
```

Events (Rust → UI), payload in parentheses:

```
ai:thinking   ({noteId, target, task})        show the reading mark / "Sketching it…"
ai:thread     (Thread)                        new or updated thread
ai:message    (Message)                       new or updated message (drawing pending → done)
ai:quiet      ({noteId, target})              read decided to say nothing
ai:error      ({noteId, task, message})
about:updated (string)                        about-you paragraph changed
chat:summary  ({noteId, summary})             the side chat's summary was folded forward in the background
```

## Trash (crates/core/src/trash.rs)

A deleted note becomes one file, `<app data>/.trash/<title> <id>.json`, which holds everything the note owns:

```
{ "duragonTrash": 1, "deletedAt", "note", "plain", "threads", "chat", "summary" }
```

The id at the end of the name, matched by the id inside, is what makes a file a note. The title is there for the person who meets it in the system trash, cut to what any file system takes. The only text from the frontend that reaches a path is a parsed `NoteId`. Nothing purges the trash.

The database wins. A file whose note is live is hidden, and no sweep removes it. That makes every order of crashes safe:

- **Delete** writes the file (temporary file, fsync, rename, fsync the folder) and reads it back inside the transaction that deletes the row. A failed write leaves the note. `Db::take_note` runs the transaction with `synchronous=FULL`, and `Db::put_note` does too.
- **Restore** commits the rows, then removes the file. A note that is already live is returned and nothing is touched. A Mind that is gone becomes the builtin one, a folder that is gone none, and a drawing still pending failed. A thread or message id that is taken aborts and keeps the file.
- **Permanent** removes any hidden file first, then deletes the rows.
- **System** does what Delete does, then hands the same file to the `SystemTrash`. Whether the file is still there afterwards says whether it went, not what the call returned, and a panic there is a refusal too. Once the note is deleted, nothing reports a failed delete. A file it refused stays in Duragon's trash. Restoring it from the system trash puts it back in `.trash/`, where the list shows it.

Every operation holds a lock on `.trash/.lock`, across processes, and can be run again from wherever it stopped. Core knows the system trash only as the `SystemTrash` trait, so its tests can't reach the real one. `src-tauri/src/system_trash.rs` implements it with the `trash` crate.

A file can be edited by anyone, so restore treats it as untrusted: drawings go through `svg::sanitize` again, sources through `clean_sources`, times are brought between 1970 and now, and nesting alone decides which note and thread a message belongs to. Links, directories, names without an id, files of another version and broken files are counted as `unreadable` and never touched. `tests/fixtures/trash-v1.json` is frozen: it must restore for as long as version 1 files exist.

The delete menu sends the mode it showed, so a setting that changes meanwhile can't turn a trip to the trash into a loss. The sidebar's Trash row opens `TrashDialog.svelte`, which reads the list again when the window gets focus.

## AI runner (crates/core/src/ai)

One function spawns the CLI and returns parsed structured output:

```
claude -p --model <m> --output-format json --json-schema <schema>
       --system-prompt <prompt> --tools <"" | "WebSearch,WebFetch">
       --setting-sources "" --strict-mcp-config --no-session-persistence --disable-slash-commands
stdin: the task payload as text
```

The result is the `structured_output` field of the JSON on stdout. The binary is `claude` on PATH, overridable with `DURAGON_CLAUDE_BIN` (tests point it at a fake script). Each child is tracked per (noteId, task) so `ai_cancel_read` can kill it. A non-zero exit or missing `structured_output` becomes `ai:error`.

When `searchMyNotes` is on, chat calls (and only chat calls) also get a notes search tool. The app binary serves it as a small MCP server over stdio when started as `duragon --notes-mcp <db path> <open note id>` (`ai/notes_mcp.rs`; `main.rs` checks for this before Tauri starts). The runner passes it inline with `--mcp-config '{"mcpServers":{"notes":{...}}}'` and adds `mcp__notes__search_notes,mcp__notes__read_note` to `--allowedTools`, since print mode denies any tool that is not pre-approved. `search_notes {query}` returns up to 8 other notes ranked by FTS5, each with id, title, last-edited date and a snippet of `plain`. `read_note {id}` returns one note's title and plain text, cut at 8000 characters. Neither ever returns the open note. The chat prompt gains a paragraph telling the Mind when to use them and to name the note it drew on by title. The server opens its own connection to the same database; WAL lets it read while the app writes.

Tasks, prompts and schemas live in `ai/prompts.rs`. Every prompt gets the Riddler (name, how it thinks, tone, frequency), the note brief, the About-you paragraph, the note title and plain text, and the open threads (so it doesn't repeat itself).

| task | trigger | output schema | follow-up |
|---|---|---|---|
| read | pause in typing, mode = thinks | `{said, kind?, quote?, message?, draw?}`; kind is `question` (default, quote is an exact substring of target) or `reply` (target is a request to the Mind, even unnamed) | creates Thread(kind=question) on the quote, or Thread(kind=reply, quote=the line) like answer; if `draw` is a non-empty brief, run draw |
| answer | a paragraph starting with or addressing "Riddled," / the riddler's name, any mode | `{message, draw?}` | creates Thread(kind=reply, quote=the line) |
| reply | user writes back in a thread | `{message, draw?}` | appends Message |
| chat | chat panel | `{message, draw?}` | appends chat Message |
| draw | after any of the above with a brief | `{svg}` | fills the Message's svg, drawing=done |
| learn | after reply/chat/answer when learnAboutYou | `{changed, aboutYou?}` | saves setting, emits about:updated |
| summarize | after a chat reply is delivered, when more than 16 chat messages are unsummarized | `{summary}` | folds the oldest 8 into the chat summary, emits chat:summary |

When `explainSymbols` is on, read, answer, reply, chat and ask (the tasks whose message the writer reads as prose) gain an optional `symbols: [{tex, meaning}]` in their schema, and their prompt gains a paragraph (`EXPLAIN_SYMBOLS`) asking for each distinct symbol and constant in the message's formulas, `tex` copied exactly as written and `meaning` a phrase of at most 8 words. When it is off, prompt and schema are unchanged. The backend trims the symbols, drops blank ones, duplicates and any whose `tex` is not in the message, keeps at most 16, and stores them on the Message. In an open thread or the side chat, `InkText` renders a formula with symbols through `renderTexWithSymbols` (`rich.ts`): `markSymbols` wraps each occurrence as `{\htmlData{sym=<i>}{…}}`, matching whole TeX tokens and skipping the arguments of `\text`, `\mathrm` and the like, and KaTeX trusts only `\htmlData`. Each symbol becomes a focusable button (`symbolTip.ts`): pointing at it with a fine pointer or focusing it shows its meaning in a tooltip, and a press writes back `Explain $<tex>$ in more detail: what does it represent here?` in the same thread or chat.

### Payload budget

Each call is stateless, so the payload is rebuilt every time, and a long note or chat would otherwise grow it without bound. The payload builders in `prompts.rs` return labelled sections (`ai/budget.rs`), not a string. A `Section` carries its text and, optionally, a shorter form and the step at which it shrinks, and a step at which it is dropped. A section with neither is never cut. `fit(sections, budget_tokens)` concatenates them and, while the estimate is over budget, applies the cuts in step order. It returns the text and whether anything was cut. Under budget the text is byte for byte what the builders produced before they returned sections (`tests/fixtures/payloads.json`).

Tokens are estimated as chars / 4, rounded up, which overcounts on purpose. Budgets are per task and tunable (constants in `budget.rs`, payload only, the system prompt not counted): read, answer, reply, ask and visualize `TASK_BUDGET` = 30 000; chat `CHAT_BUDGET` = 80 000; draw `DRAW_BUDGET` = 30 000.

Drop order (`Stage`, then rank within it):

1. Older chat messages, oldest first (in reply, older messages of the thread). A run of them reads `[… N earlier messages omitted …]`.
2. Closed (resolved or dismissed) threads in the chat payload: each first shrinks to its quote and last message, then they go, oldest first, as `[… N threads omitted …]`.
3. Distant parts of the note. The note is split into blocks (`doc::blocks`), and a block over 1000 chars into pieces at whitespace. Headings (the title and `heading` blocks) and the pieces around the focus stay, where the focus is the target for read, answer and ask (its `nth` occurrence: the copy being read, the addressed copy, the selection), visualize (its first occurrence), the thread's quote for reply, the cursor paragraph (`near`) for chat, and the start of the note when it is not found. The rest goes farthest first, each run replaced by `[… N words omitted …]`.
4. Open threads shrink to their quote and last message, and are never dropped.

Never cut: the system prompt, About you and the Riddler (all in the system prompt), the note header, the target, the writer's latest message, and the chat summary. The volatile parts (target, latest message) stay last so the stable prefix can be cached.

When a call was cut, the message it produced carries `trimmed: true` (stored in `messages.trimmed`). The thread view and the side chat show a faint line under it (`TrimmedHint.svelte`): "Read around this passage" or "Read around your cursor; the note was too long to read whole." A drawing's own payload is fitted too, but does not change its message's flag.

### Side chat summary

A long side chat keeps a rolling summary per note (`chat_summaries`, model `ChatSummary { text, through }`), where `through` is the id of the last chat message the summary covers. The chat payload sends `EARLIER IN THIS CHAT:` and the summary (when it is not empty), then only the messages after `through`, verbatim. The panel still shows the full history.

After a chat reply is delivered, never before, a background task (`Ai::summarize`, on the read model, like learn) checks the messages after the watermark. When there are more than 16 (`SUMMARIZE_AFTER`), it sends the summary so far and the oldest 8 (`SUMMARIZE_FOLD`) to the `summarize` task (`SUMMARIZE` prompt: at most about 180 words of plain text keeping decisions, conclusions, open questions and what the writer said they want). It saves the new text and the watermark (the 8th message's id) in one row, and only if the stored summary is still the one it read (`Db::fold_chat_summary`), so a run that races an edit or another run saves nothing. Then it emits `chat:summary`. One run per note at a time: a per-note claim in `Ai::summarizing`, released on drop. A failed run saves nothing and stays silent, so the next delivered reply retries.

When a summary exists, the side chat shows a collapsed "Earlier in this chat" row at the top (`EarlierInChat.svelte`). Expanded, it is an editable textarea that saves 300 ms after typing stops and on blur (`save_chat_summary`, which keeps the watermark and caps the text at 2,000 chars like a fold). Each save names the summary it edits, so a fold that lands while the box has focus refuses the stale draft instead of being overwritten, and the box then shows the fold. It follows `chat:summary` while it is not focused. The panel's Clear calls `clear_chat`, which cancels a chat reply still on its way (`Ai::clear_chat`) and deletes the side chat's messages and its summary in one transaction; margin threads stay. A summary still running when the chat is cleared saves nothing, because the message its watermark points to is gone.

The editor alone decides when a line is read; the backend reads whatever it is sent. After each settled edit, `Editor.svelte` reports every top-level block's text and which blocks are new (`changedBlocks`: blocks matching from either end are unchanged, so a pasted copy of a line is the new one). The riddle reads the last new block that is long enough and not addressed to the mind, unless an open thread is anchored in that block. So an unchanged line is never read again, a line pasted or written again is, and a copy that already holds a thread is not.

Drawing rules sent to the model: SVG only, viewBox width 320, `fill:none;stroke-linecap:round;stroke-linejoin:round` on the root, colours only through `var(--ink) var(--graphite) var(--faint) var(--rule) var(--riddle-wash)`, fonts through `var(--font-voice)` and `var(--font-sans)` at 11–18 px, slightly bowed hand-drawn paths, content grouped in `<g data-step="N" data-caption="...">` in reading order. The backend sanitizes the SVG (allow-list of elements and attributes, no scripts, no external refs, no event handlers) before storing it.

## Frontend structure

- `App.svelte`: TopBar (hamburger, blot + wordmark, Chat pill), Sidebar, Page, ChatPanel, CommandPalette, TrashDialog, CustomizeDialog, voice hint.
- `Page.svelte`: width handles, NoteHeaderRow (Riddler picker, Tell Riddled, mode, Voice, Web, Export), TellRiddledCard, Editor, Margin(s).
- Editor: TipTap with a Title node as the first block, StarterKit (headings 1–4, lists, quote), CodeBlockLowlight with a per-block language picker, HorizontalRule with `---` input rule, SlashMenu, a Decorations plugin that underlines each open thread's quote (threads quoting the same whole line take its copies in order; a shorter quote sits where it first occurs), and widget decorations that render `reply` threads inline under their line.
- Margin layout: each thread's top = the top of its quote's first line; threads are stacked to avoid overlap (push down). Both-margins placement alternates by order; right puts all on the right; marks renders a small `?` widget after the quote that opens the thread.
- Read trigger: on doc change, debounce by reading pace; target = the last new block of the settled edit (see the read rule above), sent with which occurrence of its text it is; cancel the in-flight read once its line is no longer in the note.
- Theme: `data-theme` on `<html>` from Settings.appearance (system follows `prefers-color-scheme`). Ink colour sets `--riddle`/`--riddle-ink`. Lettering sets `--font-voice`.

## Motion

Reproduce the measured timings in `reference-riddled.md` (Animation timings). All keyframes live in `styles/motion.css`; helpers in `lib/motion/`. Respect `prefers-reduced-motion` by collapsing durations to 0 and skipping the per-word reveal.

## Verification

- Rust: `cargo test -p duragon-core` covers migrations, CRUD, FTS, settings round-trip, the runner against a fake `claude` script (success, error exit, cancel), prompt assembly, and SVG sanitizing, and the trash at every point it can stop.
- UI: `npm run dev` with the mock backend, driven by agent-browser in its own headless session (`--session duragon`), never the user's Chromium.
- Desktop: `cargo tauri dev` inside `nix develop` launches the window and talks to the real CLI.

## Voice (last phase)

- Speech to text: NVIDIA Parakeet TDT 0.6B v2 (https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2, English, CC-BY-4.0), run on CPU through ONNX Runtime from Rust using an int8 ONNX export of the same model. No GPU needed. The model is downloaded on first use into the app data dir, not bundled. Hold Alt to talk, tap Alt twice for hands-free (voice activity detection ends an utterance).
- Text to speech: Kokoro-82M on CPU (ONNX), voices chosen in Customize → Voice.
- Audio capture with cpal in Rust; the webview never touches the microphone.

## Content security policy

`app.security.csp` in `src-tauri/tauri.conf.json` is the one policy. Tauri sends it with the built app, and `vite.config.ts` sends it in dev, where Tauri doesn't, plus the hot-reload socket. It allows the app's own scripts, styles and fonts, `data:` fonts and images, and IPC. Everything else is refused.

Inline styles stay allowed, because KaTeX and drawings are built from style attributes. Tauri would add a nonce to `style-src`, and browsers ignore `'unsafe-inline'` beside a nonce, so `dangerousDisableAssetCspModification` names `style-src`. Script sources are still hashed by Tauri.

A policy doesn't stop navigation, so `own_page` in `src-tauri/src/main.rs` does. The window stays on the app's own pages, and in dev on the dev server's. Sources open in the browser.
