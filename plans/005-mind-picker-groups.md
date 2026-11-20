# 005 — Mind picker: "Yours" group and a New / Manage footer

- **Status**: DONE
- **Commit**: 550d3e3
- **Severity**: LOW
- **Category**: Riddled parity (options)
- **Estimated scope**: 1 file, ~25 lines

## Problem

Riddled's Mind picker (its "Riddler for this note" listbox) has three parts:

1. the built-in mind on its own;
2. a group labelled **Yours** (`px-2.5 pb-1 pt-2 text-[12px] text-faint`) listing custom minds;
3. a footer row, `mt-1 flex items-center gap-1 border-t border-rule px-1 pt-1.5`, holding **New Riddler** (`h-8 rounded-lg px-2 text-[13px] text-graphite`, with an icon) and **Manage** (`ml-auto h-8 rounded-lg px-2 text-[13px] text-faint`). Both hover to `bg-shelf` with ink text.

Duragon lists every mind in one run, then a separator and one "New Mind…" row, and has no way to get to Customize → Minds from here:

```svelte
<!-- src/lib/components/NoteHeaderRow.svelte:42-52 — current -->
{#each notes.riddlers as r (r.id)}
  <button class="menu-item opt" ...>…</button>
{/each}
<div class="menu-sep"></div>
<button class="menu-item" role="menuitem" onclick={() => { close(); newMindForNote(); }}>
  <Plus size={15} strokeWidth={1.5} />New Mind…
</button>
```

## Target

```svelte
{#each notes.riddlers.filter((r) => r.builtin) as r (r.id)}{@render mindOption(r)}{/each}
{#if notes.riddlers.some((r) => !r.builtin)}
  <p class="group-label">Yours</p>
  {#each notes.riddlers.filter((r) => !r.builtin) as r (r.id)}{@render mindOption(r)}{/each}
{/if}
<div class="foot">
  <button class="foot-btn" role="menuitem" onclick={() => { close(); newMindForNote(); }}><Plus size={14} strokeWidth={1.5} />New Mind</button>
  <button class="foot-btn manage" role="menuitem" onclick={() => { close(); openCustomize('minds'); }}>Manage</button>
</div>
```

Here `mindOption` is a `{#snippet mindOption(r)}` holding the existing option `<button class="menu-item opt" …>` markup, moved unchanged.

```css
.group-label { padding: 8px 10px 4px; font-size: 12px; line-height: 1.3; color: var(--faint); }
.foot { display: flex; align-items: center; gap: 4px; margin-top: 4px; padding: 6px 4px 0; border-top: 1px solid var(--rule); }
.foot-btn {
  display: inline-flex; align-items: center; gap: 6px; height: 32px; padding: 0 8px;
  border-radius: 8px; font-size: 13px; color: var(--graphite);
  transition: background-color 100ms var(--ease-ui), color 100ms var(--ease-ui);
}
.foot-btn.manage { margin-left: auto; color: var(--faint); }
.foot-btn:hover, .foot-btn:focus-visible { background: var(--shelf); color: var(--ink); outline: none; }
```

## Repo conventions to follow

- `openCustomize(page)` is exported from `src/lib/state/app.svelte.ts`, and `'minds'` is a valid `CustomizePage`.
- `Riddler` has `builtin: boolean` (`src/lib/api/types.ts:71`).
- Svelte 5 snippets (`{#snippet}` / `{@render}`).
- `Popover.svelte`'s arrow-key navigation selects `[role^="menuitem"]`, so both footer buttons stay reachable by keyboard.

## Steps

1. In `src/lib/components/NoteHeaderRow.svelte`, add `openCustomize` to the import from `../state/app.svelte`.
2. Inside the riddler `<Popover>`, define `{#snippet mindOption(r: (typeof notes.riddlers)[number])}`. Move the existing option button into it. Then replace the list, separator and "New Mind…" row with the target markup.
3. Add the target CSS to the component's `<style>`.

## Boundaries

- Do NOT change how a mind is picked (`pickRiddler`) or how minds are created (`newMindForNote`).
- Do NOT touch `Popover.svelte` or the mode and export menus.
- If the code no longer matches the excerpts (drift since 550d3e3), STOP and report.

## Verification

- **Mechanical**: `npm run check` reports 0 errors.
- **Feel check**:
  - With only the built-in Duragon mind, the menu shows it, then the footer, and no "Yours" label.
  - After creating a custom mind, it appears under **Yours**.
  - **Manage** closes the menu and opens Customize on the Minds page. **New Mind** opens the mind dialog.
  - Arrow keys walk through the options and both footer buttons.
- **Done when**: the above hold and the check is clean.
