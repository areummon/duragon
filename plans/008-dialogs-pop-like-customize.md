# 008 — Thread and drawing dialogs open like the others

- **Status**: DONE (f72b87b)
- **Commit**: 10c3e97
- **Severity**: LOW
- **Category**: Cohesion & tokens
- **Estimated scope**: 2 files, 2 lines each

## Problem

Duragon opens its dialogs two different ways. The Customize and Mind dialogs fade the scrim with `veil` and scale the card in with `pop`. Exemplar:

```svelte
<!-- src/lib/components/customize/CustomizeDialog.svelte:43 — the pattern to copy -->
<div class="scrim" role="presentation" onpointerdown={...} transition:veil>
  <div class="card" ... in:pop out:pop={{ duration: 140 }}>
```

The thread dialog and the drawing lightbox instead fade the scrim and card together with `rise` at `y: 0`, which is a plain fade, so the card never scales:

```svelte
<!-- src/lib/components/ThreadDialog.svelte:14 — current -->
<div class="scrim" role="presentation" onclick={(e) => e.target === e.currentTarget && close()} transition:rise={{ y: 0, duration: 180 }}>
  <div class="card" role="dialog" aria-modal="true" aria-label="Thread" tabindex="-1">
```

```svelte
<!-- src/lib/components/DrawingDialog.svelte:13 — current -->
<div class="scrim" role="presentation" onclick={(e) => e.target === e.currentTarget && close()} transition:rise={{ y: 0, duration: 180 }}>
  <div class="card" role="dialog" aria-modal="true" aria-label="The drawing, larger" tabindex="-1">
```

## Target

- The scrim uses `transition:veil`. That is opacity over 200 ms on `cubic-bezier(0.22, 1, 0.36, 1)`.
- The card uses `in:pop out:pop={{ duration: 140 }}`. That is opacity plus scale from 0.98 to 1 over 200 ms in and 140 ms out, on the same curve.
- The card's origin stays at its center, which is correct for a modal that isn't anchored to a trigger.

## Repo conventions to follow

- `pop` and `veil` live in `src/lib/motion/transitions.ts` and already handle reduced motion. After plan 006 they fade without scaling.

## Steps

1. `src/lib/components/ThreadDialog.svelte`:
   - Change the import `import { rise } from '../motion/transitions';` to `import { pop, veil } from '../motion/transitions';`.
   - On the `.scrim` div, replace `transition:rise={{ y: 0, duration: 180 }}` with `transition:veil`.
   - On the `.card` div, add `in:pop out:pop={{ duration: 140 }}`.
2. `src/lib/components/DrawingDialog.svelte`: make the same three changes.

## Boundaries

- Do NOT change click, Escape or focus handling. Do NOT touch the dialogs' CSS.
- If `rise` is used elsewhere in either file, keep it in the import.

## Verification

- **Mechanical**: `npm run check` reports 0 errors.
- **Feel check**: open a thread in its dialog and open a drawing larger.
  - Both cards grow very slightly into place while the scrim fades, matching Customize (open it with the settings menu and compare side by side).
  - Closing is quicker than opening.
  - Slow down animations in Web Inspector (or set the durations to 2000 temporarily) and confirm that the card grows from its center.
- **Done when**: all four dialogs open and close the same way.
