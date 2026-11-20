# 001 — Make the page-width drag feel direct, like Riddled

- **Status**: DONE
- **Commit**: 550d3e3
- **Severity**: HIGH
- **Category**: Interruptibility, Missed opportunities (Riddled parity)
- **Estimated scope**: 5 files, ~60 lines

## Problem

Dragging the page-width handles (`src/lib/components/Page.svelte:51-74`) feels wrong in three ways.

1. Margin threads lag behind the drag. Each `.slot` eases every new `translateY` and every horizontal shift over 340 ms. During a drag, those positions change every pointer move, so the threads swim behind the page edge.

   ```css
   /* src/lib/components/Margin.svelte:113-118 — current */
   .slot {
     ...
     transition: transform 340ms var(--ease-step), opacity 340ms var(--ease-step);
   }
   ```

2. The note toolbar does not reflow. `.row` has a fixed `height: 30px` and every `.item` is `white-space: nowrap`. At narrow widths (520 px minimum) the items overflow the page (`src/lib/components/NoteHeaderRow.svelte:97-131`). Riddled's toolbar is `flex-wrap` and moves the whole "mode / Voice / Web" group to a second line as one unit.

3. The drag gives no readout. While the pointer is down, Riddled shows a small ink pill reading `614px` beside the dragged handle, and turns the handle's line to ink. Duragon's handle (`Page.svelte:120-139`) is a 1 px line with no active state.

Measured on riddled.ai (Tailwind classes copied from the live DOM):

- handle: `absolute -top-10 z-20 flex h-7 w-6 cursor-col-resize select-none justify-center`, with `-left-3` and `-right-3`
- handle line: `mt-1 h-3.5 w-[2px] rounded-full transition-colors duration-150 bg-rule group-hover:bg-graphite`, and `bg-ink` while dragging
- pill, only while dragging: `pointer-events-none absolute top-0 whitespace-nowrap rounded-full bg-ink px-2 py-0.5 text-[11.5px] tabular-nums text-paper right-6` (on the right handle; `left-6` mirrors it on the left handle). The pill does not animate in or out.
- toolbar row: `-ml-2 mb-3 flex flex-wrap items-center gap-x-1 gap-y-0.5`. The mode picker and the Voice and Web switches share one wrapper, `relative flex items-center gap-0.5`. Share (our Export) has `ml-auto`.
- handle a11y: `role="separator" aria-orientation="vertical" aria-valuenow aria-valuemin aria-valuemax`

## Target

- While a drag is in progress, margin slots, tethers and the toolbar follow the pointer on the same frame, with no transition. When the pointer goes up, the existing transitions come back.
- The toolbar wraps. The mode picker, Voice and Web move as one group, and Export stays right-aligned.
- The handle is 24 × 28 px. Its line is 2 × 14 px, fully rounded. It is `var(--rule)` at rest, `var(--graphite)` on hover, and `var(--ink)` while dragging. The colour transition is 150 ms `var(--ease-ui)`.
- While dragging, a pill shows the width (`614px`) next to the handle on the page side. It appears and disappears instantly, since this is direct-manipulation feedback.

## Repo conventions to follow

- Global UI state lives in `src/lib/state/layout.svelte.ts` (`export const layout = $state({...})`).
- Colour tokens: `--rule`, `--graphite`, `--ink`, `--paper`, `--faint`, `--shelf` (`src/styles/tokens.css`).
- Motion tokens: `--dur-ui: 150ms`, `--ease-ui` (`src/styles/motion.css`). Use them rather than literals.
- Svelte 5 runes (`$state`, `$derived`). There are no comments in the code unless they explain a non-obvious why.

## Steps

1. `src/lib/state/layout.svelte.ts`: add `resizing: false` to the `layout` state object, giving `export const layout = $state({ vw: window.innerWidth, scrolled: false, resizing: false });`.

2. `src/lib/components/Page.svelte`, inside the script:
   - Import `layout` from `../state/layout.svelte`.
   - Add `let dragging = $state<{ side: -1 | 1; width: number } | null>(null);`.
   - In `drag(side)`, after `setPointerCapture`, set `layout.resizing = true; dragging = { side, width: start };`.
   - In `move`, after `previewPageWidth(width)`, set `dragging = { side, width };`.
   - In `up`, set `layout.resizing = false; dragging = null;` before calling `updateSettings`.

3. `src/lib/components/Page.svelte`, markup: replace the two handle `<div>`s with handles that carry a line span and the pill:

   ```svelte
   {#each [-1, 1] as const as side (side)}
     <div
       class="handle"
       class:left={side === -1}
       class:right={side === 1}
       class:active={dragging?.side === side}
       role="separator"
       aria-orientation="vertical"
       aria-label="Page width: drag to change, double-click to reset"
       aria-valuenow={Math.round(page?.getBoundingClientRect().width ?? PAGE_WIDTH_DEFAULT)}
       aria-valuemin={PAGE_WIDTH_MIN}
       aria-valuemax={PAGE_WIDTH_MAX}
       onpointerdown={drag(side)}
       ondblclick={reset}
     >
       <span class="line" aria-hidden="true"></span>
       {#if dragging?.side === side}<span class="readout" aria-hidden="true">{dragging.width}px</span>{/if}
     </div>
   {/each}
   ```

   If `aria-valuenow` from `getBoundingClientRect()` doesn't update reactively, use `dragging?.width ?? settings.value.pageWidth ?? PAGE_WIDTH_DEFAULT` instead. Keep the `title` attribute off, because the pill replaces it.

4. `src/lib/components/Page.svelte`, styles: replace the `.handle`, `.handle.left`, `.handle.right`, `.handle::after` and `.handle:hover::after` rules with:

   ```css
   .handle {
     position: absolute;
     top: 61px;
     z-index: 20;
     display: flex;
     justify-content: center;
     width: 24px;
     height: 28px;
     cursor: col-resize;
     touch-action: none;
     user-select: none;
   }
   .handle.left { left: -12px; }
   .handle.right { right: -12px; }
   .line {
     margin-top: 4px;
     width: 2px;
     height: 14px;
     border-radius: 999px;
     background: var(--rule);
     transition: background-color var(--dur-ui) var(--ease-ui);
   }
   .handle:hover .line { background: var(--graphite); }
   .handle.active .line { background: var(--ink); }
   .readout {
     position: absolute;
     top: 0;
     padding: 2px 8px;
     border-radius: 999px;
     background: var(--ink);
     color: var(--paper);
     font-size: 11.5px;
     line-height: 1.4;
     font-variant-numeric: tabular-nums;
     white-space: nowrap;
     pointer-events: none;
   }
   .handle.right .readout { right: 24px; }
   .handle.left .readout { left: 24px; }
   @media (pointer: coarse) { .handle { display: none; } }
   ```

5. `src/lib/components/Margin.svelte`:
   - Import `layout` from `../state/layout.svelte`. `MARGIN_GAP`, `marginWidth` and `useMarks` are already imported from there, so add it to that import.
   - Add `class:resizing={layout.resizing}` to the `.margins` div.
   - Add the CSS rule `.margins.resizing .slot { transition: none; }`.

6. `src/lib/components/NoteHeaderRow.svelte`:
   - Wrap the mode `.slot`, the Voice switch and the Web switch, in that order, in one `<div class="group">…</div>`.
   - In the first `.row` rule, replace `gap: 6px; height: 30px;` with `flex-wrap: wrap; column-gap: 4px; row-gap: 2px; min-height: 30px;`.
   - Add `.group { display: flex; align-items: center; gap: 2px; }`.
   - Leave `.slot.end { margin-left: auto; }` as it is.
   - Add `.row.pill { border-radius: 18px; }` after the existing `.row.pill` rule, so a two-line sticky pill reads as a rounded card rather than a stadium.

## Boundaries

- Do NOT change `PAGE_WIDTH_MIN`, `PAGE_WIDTH_MAX`, `PAGE_WIDTH_DEFAULT`, or how width is saved (`previewPageWidth` / `updateSettings`).
- Do NOT touch the editor, riddle anchors, or `Thread.svelte`.
- Do NOT add dependencies.
- If the code no longer matches the excerpts (drift since 550d3e3), STOP and report.

## Verification

- **Mechanical**: `npm run check` reports 0 errors.
- **Feel check** (`nix develop -c cargo tauri dev`, on a note with margin threads):
  - Drag the right handle slowly, then fast. Margin threads stay glued to the page edge with no swim or catch-up. After you let go, opening a thread still eases its neighbours.
  - The pill reads the live width, sits on the page side of the handle, and vanishes on pointer-up.
  - The handle line is faint at rest, darker on hover, and ink while held.
  - At 520 px, the toolbar sits on two lines: Mind and Tell on the first; mode, Voice, Web and Export on the second. Export stays right-aligned. Nothing overflows the page.
  - Double-clicking either handle resets to 690.
- **Done when**: all of the above hold and `npm run check` is clean.
