# 003 — Open chat the way Riddled does: page slides aside, button steps away

- **Status**: DONE
- **Commit**: 550d3e3
- **Severity**: MEDIUM
- **Category**: Missed opportunities, Cohesion & tokens
- **Estimated scope**: 3 files, ~25 lines

## Problem

When chat opens, Duragon's panel slides over the page, and the page only dims in place:

```css
/* src/lib/components/ChatPanel.svelte:101-118 — current */
.panel { ... transform: translateX(100%); transition: transform 320ms var(--ease-step); visibility: hidden; }
.panel.open { transform: none; visibility: visible; }
.panel:not(.open) { transition: transform 320ms var(--ease-step), visibility 0s linear 320ms; }
```

```css
/* src/lib/components/Page.svelte:107-111 — current */
.stage { min-height: 100vh; transition: transform var(--dur-drawer) var(--ease-drawer), opacity var(--dur-drawer) var(--ease-drawer); }
.stage.dim { opacity: 0.6; }
.stage.shifted { transform: translateX(288px); opacity: 0.6; }
```

On riddled.ai (measured live at a 1896 px window):

- The page sheet moves aside, with `transform: translateX(-210px)` and `opacity: 0.5`, over `transform 0.38s cubic-bezier(0.16, 1, 0.3, 1), opacity 0.38s cubic-bezier(0.16, 1, 0.3, 1)`. The writing stays visible beside the chat instead of under it.
- The panel animates `transform` and `opacity` over the same 380 ms and curve. It is `translate-x-full opacity-0` when closed and `translate-x-0 opacity-100` when open.
- The top-bar Chat button gets `pointer-events-none scale-90 opacity-0` while chat is open, with `transition: opacity 0.2s cubic-bezier(0.4,0,0.2,1), transform 0.2s cubic-bezier(0.4,0,0.2,1)`. It steps away because the panel has its own close button.

Duragon already has these exact values as tokens: `--dur-drawer: 380ms` and `--ease-drawer: cubic-bezier(0.16, 1, 0.3, 1)` (`src/styles/motion.css`). The sidebar already uses them (`Sidebar.svelte:226-230`). The chat panel is the odd one out, on 320 ms `--ease-step`.

## Target

```css
/* ChatPanel.svelte */
.panel {
  transform: translateX(100%);
  opacity: 0;
  visibility: hidden;
  transition: transform var(--dur-drawer) var(--ease-drawer), opacity var(--dur-drawer) var(--ease-drawer), visibility 0s linear var(--dur-drawer);
}
.panel.open { transform: none; opacity: 1; visibility: visible; transition-delay: 0s; }
```

```css
/* Page.svelte */
.stage.aside { transform: translateX(-210px); }
.stage.dim { opacity: 0.5; }
.stage.shifted { transform: translateX(288px); opacity: 0.6; }   /* unchanged; sidebar wins if both are open */
```

```css
/* TopBar.svelte */
.chat { transition: opacity 200ms var(--ease-ui), transform 200ms var(--ease-ui), background-color var(--dur-ui) var(--ease-ui); }
.chat.away { opacity: 0; transform: scale(0.9); pointer-events: none; }
```

## Repo conventions to follow

- Motion tokens live only in `src/styles/motion.css`, and the drawer tokens are already there. Do not add new ones.
- Exemplar: the sidebar's transition at `src/lib/components/Sidebar.svelte:226-231` (transform + opacity + delayed visibility, with `transition-delay: 0s` on open).

## Steps

1. `src/lib/components/ChatPanel.svelte` styles: replace the three `.panel` rules quoted above with the target block. Keep all other `.panel` declarations (position, size, background, border).
2. `src/lib/components/ChatPanel.svelte` script: change the focus delay `window.setTimeout(() => input?.focus(), 320)` to `380`.
3. `src/lib/components/Page.svelte` markup: on `<main class="stage" ...>`, add `class:aside={app.chatOpen && !app.sidebarOpen && layout.vw >= 1100}`, and import `layout` from `../state/layout.svelte`. The width guard keeps the page from sliding under the 480 px panel on small windows. Below 1100 px the panel simply covers the page, as it does today.
4. `src/lib/components/Page.svelte` styles: add `.stage.aside { transform: translateX(-210px); }` before `.stage.shifted`, and change `.stage.dim { opacity: 0.6; }` to `opacity: 0.5`.
5. `src/lib/components/TopBar.svelte`: add `class:away={app.chatOpen}` to the `.chat` button. In its style rule, replace `transition: opacity var(--dur-ui) var(--ease-ui), background-color var(--dur-ui) var(--ease-ui);` with the target transition, and add the `.chat.away` rule. Also add `tabindex={app.chatOpen ? -1 : 0}` so the hidden button is not focusable.

## Boundaries

- Do NOT change the chat's content, its pin logic, or the scrim.
- Do NOT touch the sidebar transitions.
- Do NOT add new motion tokens.
- If the code no longer matches the excerpts (drift since 550d3e3), STOP and report.

## Verification

- **Mechanical**: `npm run check` reports 0 errors.
- **Feel check** (full-size window):
  - Ctrl+J or click Chat: the panel glides in from the right while the page drifts left and dims to half, both settling together on the same curve. The Chat button shrinks slightly and fades out.
  - Esc: everything reverses on the same curve. The Chat button returns.
  - Toggle chat rapidly: the transitions retarget mid-flight and never restart from zero.
  - Pin the chat (pin icon): the page stays shifted but at full opacity.
  - Narrow the window below 1100 px: the page no longer shifts, and the panel overlays it.
  - With reduced motion on, everything snaps (global rule in `motion.css`).
- **Done when**: the above hold and the check is clean.
