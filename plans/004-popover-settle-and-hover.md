# 004 — Popovers settle down from their trigger; hovers match Riddled

- **Status**: DONE
- **Commit**: 550d3e3
- **Severity**: MEDIUM
- **Category**: Physicality & origin, Cohesion
- **Estimated scope**: 3 files, ~10 lines

## Problem

Dropdowns that open below their trigger enter by moving up, toward the trigger, which reads backwards:

```svelte
<!-- src/lib/components/Popover.svelte:58 — current -->
transition:rise={{ y: up ? -4 : 4 }}
```

`rise` draws `translateY((1 - t) * y)`, so `y: 4` starts 4 px below the resting place, and the menu climbs into position. Riddled's popovers use `animation: settle 0.2s ease-out both`, with keyframe `from { opacity: 0; transform: translateY(-2px) }`. They drop the 2 px down out of the trigger. Duragon already defines that keyframe as `@keyframes settle` in `src/styles/motion.css`.

Hover feedback also differs. Riddled's menu options and toolbar buttons hover to the `bg-shelf` background, with `transition-colors duration-100` on menu options and `duration-150` on the toolbar, and toolbar buttons are `rounded-md`. Duragon's menu items hover to `--riddle-wash` (`src/styles/ui.css:26`), and the toolbar items change colour only (`src/lib/components/NoteHeaderRow.svelte:120-131`).

## Target

- `Popover.svelte`: `transition:rise={{ y: up ? 2 : -2, duration: 200 }}`. It starts 2 px toward the trigger and settles away from it over 200 ms with `quintOut`, which is the strong ease-out already used by `rise`.
- `ui.css` `.menu-item`: `transition: background-color 100ms var(--ease-ui);` and hover/focus background `var(--shelf)`.
- `NoteHeaderRow.svelte` `.item`: `border-radius: 6px;` and `.item:hover, .item[aria-expanded='true'] { color: var(--ink); background: var(--shelf); }`.

## Repo conventions to follow

- Motion helpers are in `src/lib/motion/transitions.ts`. Do not change `rise` itself, because toasts, the listening pill and the selection menu rely on its current signature.
- Colour tokens are in `src/styles/tokens.css` (`--shelf` exists in both themes).

## Steps

1. `src/lib/components/Popover.svelte:58`: replace `transition:rise={{ y: up ? -4 : 4 }}` with `transition:rise={{ y: up ? 2 : -2, duration: 200 }}`.
2. `src/styles/ui.css`, in the `.menu-item` rule: change `transition: background-color var(--dur-ui) var(--ease-ui);` to `transition: background-color 100ms var(--ease-ui);`. In `.menu-item:hover, .menu-item:focus-visible`, change `background: var(--riddle-wash);` to `background: var(--shelf);`.
3. `src/lib/components/NoteHeaderRow.svelte`, in the `.item` rule: change `border-radius: 8px;` to `border-radius: 6px;`. Change the rule `.item:hover, .item[aria-expanded='true'] { color: var(--ink); }` to also set `background: var(--shelf);`.

## Boundaries

- Do NOT edit `transitions.ts`, `SelectionMenu.svelte`, `Toast.svelte` or `ListeningPill.svelte`.
- Do NOT change popover markup.
- If the code no longer matches the excerpts (drift since 550d3e3), STOP and report.

## Verification

- **Mechanical**: `npm run check` reports 0 errors.
- **Feel check**:
  - Open the Mind, mode and Export menus: each one drops a hair down out of its button, never up into it. Slowed down 10× in devtools, the first frame sits 2 px above the final position.
  - The sidebar's settings menu (`up`) rises a hair from below.
  - Hovering menu rows and toolbar buttons shows the soft shelf background, in both light and dark themes.
- **Done when**: the above hold and the check is clean.
