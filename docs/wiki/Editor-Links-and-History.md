# Editor, Links & History

## Rich-Text Editor

Duragon's editor (`src/lib/editor/`) is built on TipTap / ProseMirror with a calm, paper-like typography:

- **Slash commands (`/`)**: Type `/` at the start of a line to insert headings, bullet/numbered lists, quotes, code blocks, math blocks, or dividers.
- **Math (KaTeX)**: Write inline `$...$` or display `$$...$$` LaTeX math with live preview. When *Explain symbols in formulas* is enabled, hover or click symbols in AI replies to see what they mean.
- **Code blocks**: Syntax-highlighted with `lowlight`, language switcher, and one-click copy.
- **Page width**: Drag the subtle handles on the left or right edge of the page to resize the writing column (double-click to reset).

## Note-to-Note `[[Links]]` & Backlinks

- Type `[[` anywhere in a note to open the inline note picker, filter by title, and press `Enter` or `Tab` to insert a quiet inline link.
- Clicking a link opens the target note.
- Target notes automatically display a **Mentioned in** footer listing every note that links to them, backed by an indexed `note_links` SQLite join table (`idx_note_links_to`) so lookups stay fast without scanning note bodies.
- **Token-efficient AI context:** When *Search my other notes in chat* is enabled, Duragon does **not** dump full linked notes into the prompt. Instead, it includes a compact index of up to 12 linked note titles and IDs so the AI can call `read_note(id)` only when it actually needs one.

## In-Note Find (`Ctrl+F`)

- Press `Ctrl+F` (`⌘F` on macOS) or `/` in Vim mode to open the floating find bar.
- Matches highlight directly in the editor via ProseMirror decorations (`FindHighlight`) with an active match indicator and counter (`1 / 4`).
- Press `Enter` / `Shift+Enter` (or `n` / `N` in Vim mode) to step through matches, and `Escape` to return focus to the editor.

## Per-Note Revision History

- Duragon automatically saves throttled snapshots (at most once every 3 minutes, deduplicated by `(title, doc_json)`, capped at 20 snapshots per note) in SQLite's `note_revisions` table.
- Click the **History** icon in the note header row to inspect earlier drafts side by side.
- Restoring any snapshot atomically snapshots your current draft first inside a single SQLite transaction, so every restore is reversible — even when restoring the 20th oldest snapshot at cap.
- Moving a note to Duragon's Trash and restoring it preserves its revision history.

## Markdown Import & Export

- **Single note export:** Click **Export** in the note header row to export as Markdown (preserving `[[Title]]` links), Word (`.docx`), or PDF (print).
- **Folder export:** Open a folder's menu in the sidebar and choose **Export folder…** to export that folder and its subfolders as a directory tree of `.md` files.
- **Full notebook export & Markdown import:** Open the sidebar **Settings** menu for **Import Markdown…** (up to 5 MB UTF-8 files, safe link sanitization, round-trip formatting) and **Export all notes…**.
- Bulk export enforces canonical root containment, rejects symlinks, never overwrites pre-existing `.md` files (`create_new`), caps filenames to 180 UTF-8 bytes, and escapes Windows reserved device names.
