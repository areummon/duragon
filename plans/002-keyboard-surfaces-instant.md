# 002 — Open keyboard-summoned surfaces instantly

- **Status**: DONE
- **Commit**: 550d3e3
- **Severity**: HIGH
- **Category**: Purpose & frequency
- **Estimated scope**: 2 files, ~4 lines

## Problem

Two surfaces that the keyboard opens many times a day fade in:

```svelte
<!-- src/lib/components/CommandPalette.svelte:61-62 — current -->
<div class="scrim" ... transition:rise|global={{ y: 0, duration: 160 }}>
  <div class="box" transition:rise|global={{ y: 0, scale: 0.97, duration: 160 }}>
```

```svelte
<!-- src/lib/editor/SlashMenu.svelte:17 — current -->
transition:rise={{ y: 4, duration: 140 }}
```

Rule: an action triggered from the keyboard and repeated 100+ times a day gets no animation. The person has already decided. A fade only puts 140–160 ms between the keypress and the list they will type into. Raycast's palette has no open or close animation. The slash menu also reopens on every `/` while writing.

## Target

Both surfaces appear and disappear on the frame they are toggled, with no `transition:` directive.

## Repo conventions to follow

- `rise` comes from `src/lib/motion/transitions.ts`. Mouse-opened popovers (`Popover.svelte`) keep using it; only these two drop it.

## Steps

1. `src/lib/components/CommandPalette.svelte`: delete `transition:rise|global={{ y: 0, duration: 160 }}` from the `.scrim` div and `transition:rise|global={{ y: 0, scale: 0.97, duration: 160 }}` from the `.box` div. Remove the now-unused `import { rise } from '../motion/transitions';`.
2. `src/lib/editor/SlashMenu.svelte`: delete `transition:rise={{ y: 4, duration: 140 }}` from the `.menu` div. Remove the now-unused `import { rise } from '../motion/transitions';`.

## Boundaries

- Do NOT change any other component's transitions.
- Do NOT change markup or styles beyond removing those directives and imports.
- If the code no longer matches the excerpts (drift since 550d3e3), STOP and report.

## Verification

- **Mechanical**: `npm run check` reports 0 errors.
- **Feel check**:
  - Press Ctrl+K: the palette is fully there on the next frame, with focus in the input. Esc removes it instantly.
  - Type `/` in the editor: the menu is there at once. Typing a filter or pressing Esc is instant.
- **Done when**: neither component imports `rise`, and the check is clean.
