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
  src/db/                 rusqlite, migrations, one module per table group
  src/ai/                 claude runner, prompts, task orchestration
  src/export/             markdown + docx from ProseMirror JSON
src-tauri/                thin shell: commands.rs maps 1:1 onto core functions, emits events
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
// trimmed: the payload behind this message was cut to its budget (see "Payload budget")
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
  inkFromPointer: boolean;
  justMyWords: boolean;                // hide all Riddled marks
  aboutYou: string;                    // <= 700 chars
  learnAboutYou: boolean;
  searchMyNotes: boolean;              // default false; side chat may search the other notes
  explainSymbols: boolean;             // default false; the Mind says what each symbol in its formulas stands for
  readModel: string;                   // default 'sonnet'  (read-along, answers, replies)
  chatModel: string;                   // default 'sonnet'
  drawModel: string;                   // default 'sonnet'
  sidebarOpen: boolean;
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
         -- trimmed: 1 when the payload was cut to its budget
riddlers(id TEXT PK, name, description, how_it_thinks, tone, frequency, builtin INT)
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
delete_note(id)
list_folders() / create_folder(name) / rename_folder(id,name) / delete_folder(id)

list_threads(noteId) -> Thread[]              includes messages
set_thread_status(id, status) / set_thread_pinned(id, pinned)
thread_to_note(id) -> Note                    "Make a note"
list_chat(noteId) -> Message[]

get_settings() -> Settings / save_settings(Settings)
list_riddlers() / save_riddler(Riddler) / delete_riddler(id)

ai_read(noteId, target)                       fire and forget; result arrives as events
ai_answer(noteId, target)
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
```

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

When `explainSymbols` is on, read, answer, reply, chat and ask (the tasks whose message the writer reads as prose) gain an optional `symbols: [{tex, meaning}]` in their schema, and their prompt gains a paragraph (`EXPLAIN_SYMBOLS`) asking for each distinct symbol and constant in the message's formulas, `tex` copied exactly as written and `meaning` a phrase of at most 8 words. When it is off, prompt and schema are unchanged. The backend trims the symbols, drops blank ones, duplicates and any whose `tex` is not in the message, keeps at most 16, and stores them on the Message. In an open thread or the side chat, `InkText` renders a formula with symbols through `renderTexWithSymbols` (`rich.ts`): `markSymbols` wraps each occurrence as `{\htmlData{sym=<i>}{…}}`, matching whole TeX tokens and skipping the arguments of `\text`, `\mathrm` and the like, and KaTeX trusts only `\htmlData`. Each symbol becomes a focusable button (`symbolTip.ts`): pointing at it with a fine pointer or focusing it shows its meaning in a tooltip, and a press writes back `Explain $<tex>$ in more detail: what does it represent here?` in the same thread or chat.

### Payload budget

Each call is stateless, so the payload is rebuilt every time, and a long note or chat would otherwise grow it without bound. The payload builders in `prompts.rs` return labelled sections (`ai/budget.rs`), not a string. A `Section` carries its text and, optionally, a shorter form and the step at which it shrinks, and a step at which it is dropped. A section with neither is never cut. `fit(sections, budget_tokens)` concatenates them and, while the estimate is over budget, applies the cuts in step order. It returns the text and whether anything was cut. Under budget the text is byte for byte what the builders produced before they returned sections (`tests/fixtures/payloads.json`).

Tokens are estimated as chars / 4, rounded up, which overcounts on purpose. Budgets are per task and tunable (constants in `budget.rs`, payload only, the system prompt not counted): read, answer, reply, ask and visualize `TASK_BUDGET` = 30 000; chat `CHAT_BUDGET` = 80 000; draw `DRAW_BUDGET` = 30 000.

Drop order (`Stage`, then rank within it):

1. Older chat messages, oldest first (in reply, older messages of the thread). A run of them reads `[… N earlier messages omitted …]`.
2. Closed (resolved or dismissed) threads in the chat payload: each first shrinks to its quote and last message, then they go, oldest first, as `[… N threads omitted …]`.
3. Distant parts of the note. The note is split into blocks (`doc::blocks`), and a block over 1000 chars into pieces at whitespace. Headings (the title and `heading` blocks) and the pieces around the focus stay, where the focus is the target for read, answer, ask and visualize, the thread's quote for reply, the cursor paragraph (`near`) for chat, and the start of the note when it is not found. The rest goes farthest first, each run replaced by `[… N words omitted …]`.
4. Open threads shrink to their quote and last message, and are never dropped.

Never cut: the system prompt, About you and the Riddler (all in the system prompt), the note header, the target, the writer's latest message, and the chat summary. The volatile parts (target, latest message) stay last so the stable prefix can be cached.

When a call was cut, the message it produced carries `trimmed: true` (stored in `messages.trimmed`). The thread view and the side chat show a faint line under it (`TrimmedHint.svelte`): "Read around this passage" or "Read around your cursor; the note was too long to read whole." A drawing's own payload is fitted too, but does not change its message's flag.

The read task never fires twice for the same unchanged target, and never when an open thread already quotes inside the target.

Drawing rules sent to the model: SVG only, viewBox width 320, `fill:none;stroke-linecap:round;stroke-linejoin:round` on the root, colours only through `var(--ink) var(--graphite) var(--faint) var(--rule) var(--riddle-wash)`, fonts through `var(--font-voice)` and `var(--font-sans)` at 11–18 px, slightly bowed hand-drawn paths, content grouped in `<g data-step="N" data-caption="...">` in reading order. The backend sanitizes the SVG (allow-list of elements and attributes, no scripts, no external refs, no event handlers) before storing it.

## Frontend structure

- `App.svelte`: TopBar (hamburger, blot + wordmark, Chat pill), Sidebar, Page, ChatPanel, CommandPalette, CustomizeDialog, voice hint.
- `Page.svelte`: width handles, NoteHeaderRow (Riddler picker, Tell Riddled, mode, Voice, Web, Export), TellRiddledCard, Editor, Margin(s).
- Editor: TipTap with a Title node as the first block, StarterKit (headings 1–4, lists, quote), CodeBlockLowlight with a per-block language picker, HorizontalRule with `---` input rule, SlashMenu, a Decorations plugin that underlines each open thread's quote, and widget decorations that render `reply` threads inline under their line.
- Margin layout: each thread's top = the top of its quote's first line; threads are stacked to avoid overlap (push down). Both-margins placement alternates by order; right puts all on the right; marks renders a small `?` widget after the quote that opens the thread.
- Read trigger: on doc change, debounce by reading pace; target = the block(s) changed since the last read; cancel any in-flight read on new input.
- Theme: `data-theme` on `<html>` from Settings.appearance (system follows `prefers-color-scheme`). Ink colour sets `--riddle`/`--riddle-ink`. Lettering sets `--font-voice`.

## Motion

Reproduce the measured timings in `reference-riddled.md` (Animation timings). All keyframes live in `styles/motion.css`; helpers in `lib/motion/`. Respect `prefers-reduced-motion` by collapsing durations to 0 and skipping the per-word reveal.

## Verification

- Rust: `cargo test -p duragon-core` covers migrations, CRUD, FTS, settings round-trip, the runner against a fake `claude` script (success, error exit, cancel), prompt assembly, and SVG sanitizing.
- UI: `npm run dev` with the mock backend, driven by agent-browser in its own headless session (`--session duragon`), never the user's Chromium.
- Desktop: `cargo tauri dev` inside `nix develop` launches the window and talks to the real CLI.

## Voice (last phase)

- Speech to text: NVIDIA Parakeet TDT 0.6B v2 (https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2, English, CC-BY-4.0), run on CPU through ONNX Runtime from Rust using an int8 ONNX export of the same model. No GPU needed. The model is downloaded on first use into the app data dir, not bundled. Hold Alt to talk, tap Alt twice for hands-free (voice activity detection ends an utterance).
- Text to speech: Kokoro-82M on CPU (ONNX), voices chosen in Customize → Voice.
- Audio capture with cpal in Rust; the webview never touches the microphone.
