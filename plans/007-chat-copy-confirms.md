# 007 — The chat's Copy button confirms

- **Status**: DONE (2fc4659)
- **Commit**: 10c3e97
- **Severity**: MEDIUM
- **Category**: Missed opportunities (feedback)
- **Estimated scope**: 1 file, about 10 lines

## Problem

The chat panel's header "Copy" button copies the whole conversation and gives no sign that it worked:

```svelte
<!-- src/lib/components/ChatPanel.svelte:63 — current -->
{#if messages.length}
  <button class="t" onclick={() => void navigator.clipboard?.writeText(chatAsText())}>Copy</button>
  <button class="t" onclick={clearChat}>Clear</button>
{/if}
```

```css
/* src/lib/components/ChatPanel.svelte:126 — current */
.t { padding: 4px 8px; font-size: 13px; line-height: 1.2; transition: color var(--dur-ui) var(--ease-ui); }
```

The code-block copy buttons already confirm with a check for 1200 ms (`src/lib/copy.ts`). This button should confirm the same way.

## Target

After a successful copy, the label reads "Copied" for 1200 ms and the button turns `var(--ink)`. The label changes in place, and the color change fades over the existing `var(--dur-ui)` (150 ms) with `var(--ease-ui)`. Nothing moves. If the clipboard write fails, nothing changes.

## Repo conventions to follow

- `src/lib/copy.ts` sets the 1200 ms confirmation (`COPIED_MS = 1200`). Reuse the same number. Don't export a new helper, because `copyFrom` swaps SVG icons and this button is text.
- Svelte 5 runes (`$state`).

## Steps

1. In the `<script>` of `src/lib/components/ChatPanel.svelte`, after `let scroller = ...`, add:
   ```ts
   let copied = $state(false);
   let copiedTimer: number | undefined;

   async function copyChat() {
     try {
       await navigator.clipboard.writeText(chatAsText());
     } catch {
       return;
     }
     copied = true;
     window.clearTimeout(copiedTimer);
     copiedTimer = window.setTimeout(() => (copied = false), 1200);
   }
   ```
2. Replace the Copy button with:
   ```svelte
   <button class="t" class:done={copied} onclick={copyChat}>{copied ? 'Copied' : 'Copy'}</button>
   ```
3. In the `<style>`, after the `.t:hover, .i:hover, ...` rule, add:
   ```css
   .t.done { color: var(--ink); }
   ```

## Boundaries

- Do NOT change `copy.ts`, the Clear button or the header layout.
- Do NOT animate position or size. The text swap is instant, and only the color fades.

## Verification

- **Mechanical**: `npm run check` reports 0 errors.
- **Feel check**: open the chat, send a message, then click Copy.
  - The label reads "Copied" in ink color, then goes back to "Copy" about a second later.
  - Pasting gives the conversation.
  - Clicking twice quickly keeps "Copied" showing for 1200 ms after the second click.
- **Done when**: the button confirms a successful copy and stays unchanged on failure.
