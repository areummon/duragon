# 011 — Sidebar menus fade between their views

- **Status**: TODO
- **Commit**: 10c3e97
- **Severity**: LOW
- **Category**: Missed opportunities (preventing a jarring change)
- **Estimated scope**: 1 file, about 6 lines

## Problem

A note's menu in the sidebar has three views inside one popover: main, "Move to folder" and the delete confirmation. A folder's menu has two. Picking an option swaps the contents in the same frame, and the popover's height jumps:

```svelte
<!-- src/lib/components/Sidebar.svelte:184 — current (abridged) -->
<Popover open={noteMenu !== null} ...>
  {#if noteMenu}
    {@const id = noteMenu.id}
    {#if noteMenu.view === 'main'}
      ...
    {:else if noteMenu.view === 'move'}
      ...
    {:else}
      ...
    {/if}
  {/if}
</Popover>

<!-- src/lib/components/Sidebar.svelte:205 — current (abridged) -->
<Popover open={folderMenu !== null} ...>
  {#if folderMenu}
    {@const id = folderMenu.id}
    {#if !folderMenu.confirm}
      ...
    {:else}
      ...
    {/if}
  {/if}
</Popover>
```

## Target

The new view fades in over 150 ms on `cubic-bezier(0.22, 1, 0.36, 1)`, using the existing `veil` transition. Only the incoming view fades. The outgoing view disappears at once, so the two never overlap. The height still changes instantly. A height animation isn't worth it in a 200px menu.

## Repo conventions to follow

- `veil` lives in `src/lib/motion/transitions.ts`. `Sidebar.svelte` already imports `unfold` from that file.
- Exemplar of a fade on a view swap is `src/lib/components/customize/CustomizeDialog.svelte:55`, which uses `<div class="page" in:rise={{ y: 4, duration: 180 }}>`.

## Steps

1. In `src/lib/components/Sidebar.svelte`, change the import `import { unfold } from '../motion/transitions';` to `import { unfold, veil } from '../motion/transitions';`.
2. Inside the note menu's `{#if noteMenu}`, wrap the `{#if noteMenu.view === 'main'} ... {/if}` chain like this:
   ```svelte
   {#key noteMenu.view}
     <div class="view" in:veil={{ duration: 150 }}>
       {#if noteMenu.view === 'main'}
         ...unchanged...
       {/if}
     </div>
   {/key}
   ```
3. Inside the folder menu's `{#if folderMenu}`, wrap its `{#if !folderMenu.confirm} ... {/if}` the same way with `{#key folderMenu.confirm}`.
4. Add to `<style>`:
   ```css
   .view { display: contents; }
   ```
   The menu items keep their existing layout. If `display: contents` stops the fade from showing (an element with `display: contents` has no box to fade), use `.view { display: flex; flex-direction: column; }` instead.

## Boundaries

- Do NOT change `Popover.svelte` or the popover's own open and close transition.
- Do NOT animate the height.
- Do NOT add an `out:` transition.

## Verification

- **Mechanical**: `npm run check` reports 0 errors.
- **Feel check**: open a note's "..." menu in the sidebar and click "Move to folder", then "Delete".
  - The new options fade in quickly, and the old ones never show through.
  - Opening the menu itself still uses the popover's normal rise, with no double fade on first open.
  - Keyboard focus and arrow navigation in the menu still work.
- **Done when**: both menus fade between views, and nothing else about them changed.
