# Keyboard & Vim Mode

All app shortcuts can be viewed and rebound in **Customize → Keyboard**. Custom bindings are persisted in `localStorage` (`duragon:keymap`).

## Default Shortcuts

| Action | Default (Linux / Windows) | Default (macOS) |
| :--- | :--- | :--- |
| Find or start a note | `Ctrl+K` | `⌘K` |
| New note | `Ctrl+N` | `⌘N` |
| Open or close chat | `Ctrl+J` | `⌘J` |
| Open settings (Customize) | `Ctrl+,` | `⌘,` |
| Find in this note | `Ctrl+F` | `⌘F` |
| Copy this note | `Ctrl+Shift+C` | `⌘⇧C` |
| Export this note | `Ctrl+Shift+E` | `⌘⇧E` |
| Just my words (hide marks) | `Ctrl+Shift+.` | `⌘⇧.` |
| Ask Mind about selection | `Ctrl+Enter` | `⌘↵` |
| Push-to-talk / Hands-free voice | Hold `Alt` / Double-tap `Alt` | Hold `⌥` / Double-tap `⌥` |

Reserved keys (`Escape`, `Tab`, `Shift+Tab`, and bare unmodified letters) cannot be bound as app shortcuts so normal typing and dialog dismissal stay predictable.

## Duragon-Tailored Vim Mode

Enable **Vim keybindings** in **Customize → Keyboard**. A quiet mode indicator (`NORMAL`, `INSERT`, `VISUAL`) appears at the bottom-left while editing and automatically hides when the sidebar or any dialog is open.

### Modes & Entry

| Key | Action |
| :--- | :--- |
| `Esc` / `Ctrl+[` | Return to Normal mode (also clears find highlights if active) |
| `i` / `a` | Insert before / after cursor |
| `I` / `A` | Insert at first non-space / end of line |
| `o` / `O` | Open new paragraph below / above |
| `v` | Enter Visual selection mode |

### Motions

| Key | Action |
| :--- | :--- |
| `h` `j` `k` `l` | Left, down, up, right |
| `w` `b` `e` | Next word start, previous word start, word end |
| `0` `^` `$` | Line start, first non-space, line end |
| `gg` / `G` | Top / bottom of note |

### Editing & Search

| Key | Action |
| :--- | :--- |
| `x` | Delete character (or selected block node) |
| `dd` / `cc` / `yy` | Delete / change / yank current line (or list item inside a list) |
| `dw` / `d$` / `D` | Delete word / to end of line |
| `cw` / `c$` / `C` | Change word / to end of line |
| `p` / `P` | Paste after / before |
| `u` / `Ctrl+R` | Undo / Redo |
| `/` | Open in-note find (`Enter` locks query and returns to Normal mode) |
| `n` / `N` | Jump to next / previous match |

### Duragon Chords (Normal Mode)

| Chord | Action |
| :--- | :--- |
| `gc` | Toggle side chat |
| `gs` | Open Customize settings |
| `gw` | Toggle Web access for current note |
| `gv` | Toggle Voice for current note |
| `gm` | Switch reading mode (*Thinks with you* ↔ *When you ask*) |
| `za` | Toggle *Just my words* |
