# 009 — Your sent chat message settles in

- **Status**: TODO
- **Commit**: 10c3e97
- **Severity**: LOW
- **Category**: Missed opportunities (spatial consistency)
- **Estimated scope**: 2 files, about 6 lines

## Problem

When you send a chat message, your bubble appears instantly above the compose box, while the reply below it inks in word by word. The bubble has no animation:

```svelte
<!-- src/lib/components/ChatPanel.svelte:80 — current -->
{#if m.author === 'you'}
  <div class="you"><InkText text={m.text} /></div>
```

```css
/* src/lib/components/ChatPanel.svelte:144 — current */
.you { align-self: flex-end; max-width: 85%; padding: 8px 14px; border-radius: 16px; background: var(--shelf); font-size: 14px; line-height: 1.5; }
```

The message is pushed in `send()` with a `local-` id:

```ts
// src/lib/state/chat.svelte.ts:52 — current
chat.messages.push({
  id: `local-${++seq}`, noteId: chat.noteId, threadId: null, author: 'you', text: t, drawing: 'none', svg: null, sources: [], createdAt: Date.now(),
});
```

When the backend echoes it, `loadChat`'s listener replaces that entry in place (`chat.messages[i] = m`), giving it the server id. The keyed `{#each}` then sees a new key and remounts the bubble.

## Target

A bubble you just sent fades in and rises 2px, using the existing `ink-in` keyframe (`opacity 0, translateY(2px)` to settled). It takes `var(--dur-ink-in)` (240 ms) on `var(--ease-step)` (`cubic-bezier(0.22, 1, 0.36, 1)`). Messages loaded from history, or shown when the panel reopens, don't animate. The bubble must not animate a second time when the backend's echo replaces the local entry.

## Repo conventions to follow

- `chat.fresh` (`src/lib/state/chat.svelte.ts:11`) already marks replies that should ink in, and `loadChat` resets it. Reuse it for your own messages.
- Exemplar of a fresh-gated entrance is `src/lib/components/InkText.svelte:65` with `animate={!!chat.fresh[m.id]}`.

## Steps

1. In `src/lib/state/chat.svelte.ts` `send()`, keep the id in a variable and mark it fresh:
   ```ts
   const id = `local-${++seq}`;
   chat.messages.push({
     id, noteId: chat.noteId, threadId: null, author: 'you', text: t, drawing: 'none', svg: null, sources: [], createdAt: Date.now(),
   });
   chat.fresh[id] = true;
   ```
2. In the `ai:message` listener in `loadChat`, the replace branch (`if (i >= 0) { ... chat.messages[i] = m; }`), keep the existing first line and before `chat.messages[i] = m;` add:
   ```ts
   if (chat.messages[i].author === 'you') {
     delete chat.fresh[chat.messages[i].id];
     chat.fresh[`echo${m.id}`] = true;
   }
   ```
   The `echo` key records that this message has already been shown. It is checked in step 3.
3. In `src/lib/components/ChatPanel.svelte`, change the bubble to:
   ```svelte
   <div class="you" class:in={!!chat.fresh[m.id]}><InkText text={m.text} /></div>
   ```
   Then confirm by running the app. If the bubble animates twice (once on send and again when the echo arrives), the echo is remounting it. In that case change the `{#each messages as m (m.id)}` key to `(m.createdAt + m.author)` only if `createdAt` is preserved by the echo. Otherwise STOP and report.
4. In the `<style>`, after `.you`, add:
   ```css
   .you.in { animation: ink-in var(--dur-ink-in) var(--ease-step) both; }
   ```

## Boundaries

- Do NOT animate the reply or the waiting blot. They already ink in.
- Do NOT add a new keyframe or token. `ink-in` and `--dur-ink-in` exist in `src/styles/motion.css`.
- Reduced motion is covered by plan 006, which turns `ink-in` into a fade. Add nothing here for it.

## Verification

- **Mechanical**: `npm run check` reports 0 errors.
- **Feel check**: open the chat and send a message.
  - Your bubble fades up into place once and doesn't flicker when the reply begins.
  - Close and reopen the panel, and the existing bubbles appear without animating.
  - Switch notes and back, and no bubble animates.
  - Use Web Inspector's animation slowdown and confirm the bubble rises only 2px.
- **Done when**: only a just-sent bubble animates, and it animates exactly once.
