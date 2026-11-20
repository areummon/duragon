# 006 — Reduced motion keeps fades, drops movement

- **Status**: DONE (810daba)
- **Commit**: 10c3e97
- **Severity**: MEDIUM
- **Category**: Accessibility
- **Estimated scope**: 1 CSS file rewritten in one block, 9 components get a small `@media` block, 2 TS files get a branch. Roughly 90 lines.

## Problem

With "reduce motion" turned on in the OS, Duragon turns off every animation and transition, fades included. Panels, menus, toasts, dialogs and replies all appear and vanish with no transition. Reduced motion should mean fewer and gentler animations, not none at all. The fix is to keep changes in opacity and color, and drop anything that travels, scales, rotates or wipes.

The global kill switch:

```css
/* src/styles/motion.css:149 — current */
@media (prefers-reduced-motion: reduce) {
  :root {
    --dur-settle: 0ms; --dur-tether: 0ms; --dur-tether-2: 0ms;
    --delay-tether: 0ms; --delay-tether-2: 0ms; --dur-quote: 0ms;
    --dur-reply-in: 0ms; --dur-reply-underline: 0ms; --dur-ink-wait: 0ms;
    --dur-ink-word: 0ms; --delay-ink-word: 0ms; --dur-blot-splat: 0ms;
    --dur-ink-dry: 0ms; --dur-ink-sheen: 0ms; --delay-ink-sheen: 0ms;
    --dur-ink-drip: 0ms; --delay-ink-drip: 0ms; --dur-step: 0ms; --dur-ui: 0ms;
    --dur-ink-in: 0ms; --dur-unfold: 0ms; --dur-pencil: 0ms; --dur-quiet-mark: 0ms;
    --dur-breathe: 0ms; --dur-caret: 0ms; --dur-code-in: 0ms; --dur-sketch: 0ms;
  }
  *, *::before, *::after {
    animation-duration: 0ms !important;
    animation-delay: 0ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0ms !important;
  }
}
```

The Svelte transitions also drop to zero:

```ts
// src/lib/motion/transitions.ts:4, 15, 25, 54, 62 — current
const reduced = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;
// rise:   duration: reduced() ? 0 : duration,  css: opacity + translateY + scale
// unfold: duration: reduced() ? 0 : duration,  css: height + opacity
// pop:    duration: reduced() ? 0 : duration,  css: opacity + scale
// veil:   duration: reduced() ? 0 : duration,  css: opacity only
```

These components already handle reduced motion correctly, so leave them alone:
- `src/lib/components/Tether.svelte:74` stops the stroke draw.
- `src/lib/components/ReadingPencil.svelte:162` stops the pencil, the breathing blot and the quiet mark.
- `src/styles/editor.css:16` keeps the caret solid.
- `src/lib/motion/reflow.ts:22` skips the FLIP glide.
- `src/lib/components/InkDrops.svelte:8` skips the decorative pointer ink.

## Target

When reduced motion is on:

1. Every opacity and color animation keeps its normal duration, curve and delay. This includes the word-by-word ink-in of replies, the ink-dry fade and the hover color changes.
2. Every keyframe that moves something keeps only its opacity part. A keyframe that only moves, with nothing to fade, holds still.
3. Every CSS transition that includes `transform`, `rotate` or `padding` drops that property, so the element jumps to its new place while its opacity still fades.
4. `rise` and `pop` keep their duration but only animate opacity. `veil` is unchanged. `unfold` stays instant, because growing height pushes content around.

## Repo conventions to follow

- Motion tokens and keyframes live in `src/styles/motion.css`. Component overrides live in each component's `<style>` block.
- Exemplar of a per-component reduced block, `src/lib/components/Tether.svelte:74`:
  ```css
  @media (prefers-reduced-motion: reduce) {
    .reveal { stroke-dashoffset: 0; animation: none; }
  }
  ```
- `@keyframes` rules inside `@media (prefers-reduced-motion: reduce)` override the same names defined earlier in the file. This is standard CSS and works in WebKitGTK.

## Steps

1. **`src/styles/motion.css`.** Replace the whole `@media (prefers-reduced-motion: reduce) { ... }` block at the end of the file (line 149 to the end) with this:

   ```css
   /* Reduced motion keeps fades and color, and holds still anything that travels, grows or wipes. */
   @media (prefers-reduced-motion: reduce) {
     @keyframes settle { from { opacity: 0; } to { opacity: 1; } }
     @keyframes ink-in { from { opacity: 0; } to { opacity: 1; } }
     @keyframes code-in { from { opacity: 0; } to { opacity: 1; } }
     @keyframes unfold { from { opacity: 0; } to { opacity: 1; } }
     @keyframes reply-underline { from { opacity: 0; } to { opacity: 1; } }
     @keyframes blot-splat { from { opacity: 0; } to { opacity: 1; } }
     @keyframes sketch-grow { from { opacity: 0; } to { opacity: 1; } }
     @keyframes sketch-pop { from { opacity: 0; } to { opacity: 1; } }
     @keyframes sketch-dot { from { opacity: 0; } to { opacity: 1; } }
     @keyframes tether-draw {
       from { stroke-dashoffset: 0; opacity: 0; }
       to { stroke-dashoffset: 0; opacity: 1; }
     }
     @keyframes ink-wait {
       0%, 100% { opacity: 0.45; }
       50% { opacity: 0.95; }
     }
     @keyframes reading-breathe {
       0%, 100% { opacity: 0.25; }
       50% { opacity: 0.85; }
     }
     @keyframes hearing {
       0%, 100% { opacity: 0.5; }
       50% { opacity: 1; }
     }
     @keyframes quiet-mark {
       0% { opacity: 0; }
       15%, 70% { opacity: 1; }
       100% { opacity: 0; }
     }
     @keyframes ink-drip { from, to { opacity: 0; } }
     @keyframes voice-bar { from, to { transform: scaleY(0.6); } }
   }
   ```

   This deletes the token zeroing and the `*, *::before, *::after` rule. Do not touch any `@keyframes` above the block.

2. **`src/lib/motion/transitions.ts`.** Change `rise`, `pop` and `unfold` as below. Leave `veil` and `bezier` unchanged except that `veil` keeps its duration under reduced motion.

   ```ts
   export function rise(_node: Element, { y = 4, scale = 1, duration = 160 }: RiseOptions = {}): TransitionConfig {
     const still = reduced();
     return {
       duration,
       easing: quintOut,
       css: (t) => (still ? `opacity:${t}` : `opacity:${t};transform:translateY(${(1 - t) * y}px) scale(${scale + (1 - scale) * t})`),
     };
   }
   ```
   `unfold` keeps `duration: reduced() ? 0 : duration` unchanged.
   ```ts
   export function pop(_node: Element, { duration = 200, from = 0.98 }: { duration?: number; from?: number } = {}): TransitionConfig {
     const still = reduced();
     return {
       duration,
       easing: easeStep,
       css: (t) => (still ? `opacity:${t}` : `opacity:${t};transform:scale(${from + (1 - from) * t})`),
     };
   }

   export function veil(_node: Element, { duration = 200 }: { duration?: number } = {}): TransitionConfig {
     return { duration, easing: easeStep, css: (t) => `opacity:${t}` };
   }
   ```

3. **`src/lib/components/Drawing.svelte:86`.** The hide fade is opacity only, so let it run. Change
   `if (animated && !reduced()) g.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 150 });`
   to
   `if (animated) g.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 150 });`.

   In `reveal` (line 72), keep the opacity fade and skip only the stroke draw. Replace
   ```ts
   if (!animated || reduced()) return;
   g.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 450, easing: 'cubic-bezier(0.22, 1, 0.36, 1)' });
   ```
   with
   ```ts
   if (!animated) return;
   g.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 450, easing: 'cubic-bezier(0.22, 1, 0.36, 1)' });
   if (reduced()) return;
   ```

4. **Per-component transitions.** Append each block at the end of that component's `<style>`. Each one restates the element's `transition` without the moving property.

   - `src/lib/components/Sidebar.svelte` (`.sidebar` at line ~232):
     ```css
     @media (prefers-reduced-motion: reduce) {
       .sidebar { transition: opacity var(--dur-drawer) var(--ease-drawer), visibility 0s linear var(--dur-drawer); }
       .sidebar.open { transition-delay: 0s; }
     }
     ```
   - `src/lib/components/ChatPanel.svelte` (`.panel` at line 117):
     ```css
     @media (prefers-reduced-motion: reduce) {
       .panel { transition: opacity var(--dur-drawer) var(--ease-drawer), visibility 0s linear var(--dur-drawer); }
       .panel.open { transition-delay: 0s; }
     }
     ```
   - `src/lib/components/Page.svelte` (`.stage` at line 135):
     ```css
     @media (prefers-reduced-motion: reduce) {
       .stage { transition: opacity var(--dur-drawer) var(--ease-drawer); }
     }
     ```
   - `src/lib/components/Margin.svelte` (`.slot` at line 117). The existing `.margins.resizing .slot { transition: none; }` must still win while resizing, so the block repeats it after the override. Do not use `!important`:
     ```css
     @media (prefers-reduced-motion: reduce) {
       .slot { transition: opacity 340ms var(--ease-step); }
       .margins.resizing .slot { transition: none; }
     }
     ```
   - `src/lib/components/Thread.svelte` (`.thread` at line 242, the reply underline `.write::after` at line 345, the pin icon at line 395):
     ```css
     @media (prefers-reduced-motion: reduce) {
       .thread { transition: opacity var(--dur-settle) var(--ease-settle); }
       .thread.leaving { transform: none; }
       .write::after { transform: scaleX(1); transition: opacity 180ms ease-out; }
       .icon :global(.lucide-pin) { transition: none; }
     }
     ```
     Check the selector for the underline: it is the rule whose body contains `transform: scaleX(0);` and `transition: transform var(--dur-reply-underline) ...`. Use that exact selector in place of `.write::after` if it differs.
   - `src/lib/components/TopBar.svelte` (`.swap` at line 69, `.chat` at line 92, `.brand` at line 48):
     ```css
     @media (prefers-reduced-motion: reduce) {
       .swap { transform: none; transition: opacity 200ms var(--ease-ui); }
       .chat { transition: opacity 200ms var(--ease-ui), background-color var(--dur-ui) var(--ease-ui); }
       .chat.away { transform: none; }
       .brand { transition: border-color 200ms var(--ease-ui), background-color 200ms var(--ease-ui); }
     }
     ```
   - `src/lib/components/Toggle.svelte` (`.knob` at line 25) and `src/lib/components/customize/Switch.svelte` (`.knob` at line 27), the same block in each:
     ```css
     @media (prefers-reduced-motion: reduce) {
       .knob { transition: background-color 180ms var(--ease-ui); }
     }
     ```

## Boundaries

- Do NOT touch `Tether.svelte`, `ReadingPencil.svelte`, `editor.css`, `reflow.ts` or `InkDrops.svelte`. They already handle reduced motion.
- Do NOT change `ListeningPill.svelte:43` (live voice level) or `VoiceModelsPage.svelte:64` (download progress bar). They show live data, so they keep moving.
- Do NOT change any animation outside reduced motion. With the setting off, the app must look exactly as before.
- Do NOT add dependencies.
- If a cited line doesn't match what you find, STOP and report instead of improvising.

## Verification

- **Mechanical**: `npm run check` reports 0 errors. `grep -n "0ms !important" src/styles/motion.css` prints nothing.
- **Feel check**: run the app with reduced motion on. GNOME users can run `gsettings set org.gnome.desktop.interface enable-animations false`, which WebKitGTK maps to `prefers-reduced-motion: reduce`. Restore it afterwards. Then confirm each of these:
  - The sidebar and chat drawer fade in place over 380 ms with no slide.
  - Popovers, the toast and dialogs fade with no rise or scale.
  - A margin reply inks in word by word, and the thinking blot pulses in opacity without growing.
  - Toggle knobs snap to their side while the track color still fades.
  - The menu and close icons in the top bar crossfade without rotating.
  - Turn the setting back on, and every motion looks exactly as it did before.
- **Done when**: with reduced motion on, nothing on screen changes position, size or rotation over time except the voice level and the progress bar, and every fade still plays.
