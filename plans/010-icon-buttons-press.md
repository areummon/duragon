# 010 — Icon buttons answer a press

- **Status**: DONE (bd7e364)
- **Commit**: 10c3e97
- **Severity**: LOW
- **Category**: Missed opportunities (feedback)
- **Estimated scope**: 2 files, about 12 lines

## Problem

No element in Duragon has a `:active` style (`grep -rn ":active" src` finds only `class:active` bindings). The send arrow, the chat header icons and the top bar's menu button react to hover but give no sign of being pressed.

```css
/* src/lib/components/ChatPanel.svelte:128 — current */
.i { display: grid; place-items: center; width: 28px; height: 28px; transition: color var(--dur-ui) var(--ease-ui); }
/* src/lib/components/ChatPanel.svelte:152-165 — current (abridged) */
.go { ... border-radius: 50%; background: var(--ink); color: var(--paper); transition: opacity var(--dur-ui) var(--ease-ui); }
.go:disabled { opacity: 0.3; cursor: default; }
```

```css
/* src/lib/components/TopBar.svelte:52 — current (abridged) */
.hamburger { position: relative; width: 32px; height: 32px; ...; transition: opacity var(--dur-ui) var(--ease-ui); }
/* src/lib/components/TopBar.svelte:81-95 — current (abridged) */
.chat { ...; transition: opacity 200ms var(--ease-ui), transform 200ms var(--ease-ui), background-color var(--dur-ui) var(--ease-ui); }
.chat.away { opacity: 0; transform: scale(0.9); pointer-events: none; }
```

## Target

While pressed, each of these scales to 0.97 and springs back on release over 160 ms on `var(--ease-step)` (`cubic-bezier(0.22, 1, 0.36, 1)`). These buttons are pressed tens of times a day, so the motion stays subtle. With reduced motion on, there is no scale.

## Repo conventions to follow

- Use `var(--ease-step)` from `src/styles/motion.css:8`, not a new curve.
- Add `transform` to each element's existing `transition` list instead of writing a second `transition` declaration.

## Steps

1. `src/lib/components/ChatPanel.svelte`:
   - `.i`: change its transition to `transition: color var(--dur-ui) var(--ease-ui), transform 160ms var(--ease-step);`.
   - `.go`: change its transition to `transition: opacity var(--dur-ui) var(--ease-ui), transform 160ms var(--ease-step);`.
   - Add:
     ```css
     .i:active, .go:active:not(:disabled) { transform: scale(0.97); }
     ```
2. `src/lib/components/TopBar.svelte`:
   - `.hamburger`: change its transition to `transition: opacity var(--dur-ui) var(--ease-ui), transform 160ms var(--ease-step);`.
   - `.chat` already transitions `transform` (200 ms, `--ease-ui`). Leave that line alone.
   - Add:
     ```css
     .hamburger:active, .chat:active { transform: scale(0.97); }
     ```
3. In both files, add or extend the reduced-motion block at the end of `<style>`:
   ```css
   @media (prefers-reduced-motion: reduce) {
     .i:active, .go:active { transform: none; }         /* ChatPanel */
     .hamburger:active, .chat:active { transform: none; } /* TopBar */
   }
   ```
   If plan 006 already added a reduced block to the file, put these lines inside it.

## Boundaries

- Do NOT add press scale to `.menu-item`, list rows or text buttons. Menus close on click, so the press would never be seen.
- Do NOT change hover styles.

## Verification

- **Mechanical**: `npm run check` reports 0 errors.
- **Feel check**: press and hold each button, then release.
  - The button shrinks very slightly while held and returns smoothly.
  - A fast click still reads as a click, with no wobble or lag before the action.
  - A disabled send button doesn't react.
  - With reduced motion on, nothing scales.
- **Done when**: the four buttons respond to a press, and nothing else changed.
