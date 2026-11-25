<script lang="ts">
  import { COPY_ICON, copyFrom } from '../copy';
  import type { MathSymbol } from '../api/types';
  import { highlight, onKatexReady, onLowlightReady, parseReply, renderTex, renderTexWithSymbols } from '../rich';
  import { symbolButtons, symbolTips } from '../symbolTip';

  /**
   * copyable: false where the reply sits inside a button, which cannot hold another.
   * symbols with onask: the symbols in its formulas explain themselves when pointed at, and pressing one asks about it.
   */
  interface Props { text: string; animate?: boolean; copyable?: boolean; symbols?: MathSymbol[]; onask?: (s: MathSymbol) => void }
  let { text, animate = false, copyable = true, symbols = [], onask }: Props = $props();

  let richRev = $state(0);
  $effect(() => {
    const offK = onKatexReady(() => richRev++);
    const offL = onLowlightReady(() => richRev++);
    return () => {
      offK();
      offL();
    };
  });

  const tips = $derived(onask && symbols.length ? { symbols, onask } : null);
  const tex = (source: string, display: boolean) =>
    tips ? symbolButtons(renderTexWithSymbols(source, display, symbols.map((s) => s.tex)), symbols) : renderTex(source, display);

  type Token = { kind: 'space' | 'word' | 'code' | 'math'; text: string; order: number };
  type Piece =
    | { kind: 'para'; tokens: Token[] }
    | { kind: 'code'; html: string; code: string; order: number }
    | { kind: 'math'; html: string; order: number };

  // Words ink in one by one; inline code, a formula and a whole block each ink in as one token.
  const pieces = $derived.by(() => {
    void richRev;
    let order = 0;
    return parseReply(text).map((b): Piece => {
      if (b.kind === 'code') return { kind: 'code', html: highlight(b.code, b.lang), code: b.code, order: order++ };
      if (b.kind === 'math') return { kind: 'math', html: tex(b.tex, true), order: order++ };
      const tokens: Token[] = [];
      for (const x of b.inlines) {
        if (x.kind === 'code') tokens.push({ kind: 'code', text: x.text, order: order++ });
        else if (x.kind === 'math') tokens.push({ kind: 'math', text: tex(x.text, false), order: order++ });
        else for (const w of x.text.split(/(\s+)/)) {
          if (!w) continue;
          tokens.push(/^\s+$/.test(w) ? { kind: 'space', text: w, order } : { kind: 'word', text: w, order: order++ });
        }
      }
      return { kind: 'para', tokens };
    });
  });

  const delay = (order: number) => (animate ? `calc(var(--delay-ink-word) * ${order})` : null);

  // The reply may sit in a clickable thread, so the copy press must not reach it.
  function copy(e: Event, code: string) {
    e.stopPropagation();
    e.preventDefault();
    void copyFrom(e.currentTarget as HTMLElement, code);
  }
</script>

<!-- Spans throughout, display:block where needed, so a reply stays valid inside the <p>, <span> and <button> that hold it. -->
{#each pieces as p, i (i)}{#if p.kind === 'para'}<span class="para"
      >{#each p.tokens as t, j (j)}{#if t.kind === 'space'}{t.text}{:else if t.kind === 'code'}<code
            class:w={animate}
            style:animation-delay={delay(t.order)}>{t.text}</code
          >{:else if t.kind === 'math'}<span class="tex" class:w={animate} style:animation-delay={delay(t.order)} use:symbolTips={tips}
            >{@html t.text}</span
          >{:else if animate}<span class="w" style:animation-delay={delay(t.order)}>{t.text}</span
          >{:else}{t.text}{/if}{/each}</span
    >{:else if p.kind === 'code'}<span class="code-wrap" class:in={animate} style:animation-delay={delay(p.order)}
      ><span class="block code-block"><code>{@html p.html}</code></span
      >{#if copyable}<span class="code-tools"
          ><span class="code-copy" role="button" tabindex="0" aria-label="Copy code" title="Copy"
            onclick={(e) => copy(e, p.code)}
            onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && copy(e, p.code)}>{@html COPY_ICON}</span
        ></span>{/if}</span
    >{:else}<span class="block math-block" class:in={animate} style:animation-delay={delay(p.order)} use:symbolTips={tips}
      >{@html p.html}</span
    >{/if}{/each}

<style>
  .w { animation: ink-word var(--dur-ink-word) var(--ease-ink) both; }
  .in { animation: ink-in var(--dur-ink-word) var(--ease-ink) both; }
  .para + .para::before { content: ''; display: block; height: var(--ink-para-gap, 0.7em); }
  code {
    font-family: ui-monospace, 'JetBrains Mono', monospace;
    font-size: 0.86em;
    padding: 0.08em 0.32em;
    border-radius: 4px;
    background: var(--code);
  }
  .tex { white-space: nowrap; }
  .block {
    display: block;
    max-width: 100%;
    margin: 0.5em 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: thin;
    scrollbar-color: var(--rule) transparent;
  }
  .block:first-child { margin-top: 0.15em; }
  /* The copy button sits on this wrapper, outside the block's horizontal scroll. */
  .code-wrap { position: relative; display: block; margin: 0.5em 0; }
  .code-wrap .block { margin: 0; }
  .code-wrap:first-child { margin-top: 0.15em; }
  .code-wrap:hover .code-tools, .code-wrap:focus-within .code-tools { opacity: 1; }
  .code-tools { top: 4px; right: 4px; border-radius: 6px; background: var(--code); }
  .code-block {
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--code);
    text-align: left;
    white-space: pre;
  }
  .code-block code {
    display: block;
    width: max-content;
    padding: 0;
    background: none;
    font-size: 12.5px;
    line-height: 1.55;
    color: var(--ink);
    tab-size: 2;
  }
  .math-block { text-align: center; padding: 0.1em 0; }
  .math-block :global(.katex-display) { margin: 0; }
  /* A dotted rule under each symbol that explains itself; a wash while it is pointed at or focused. */
  .tex :global([data-sym]), .math-block :global([data-sym]) {
    cursor: pointer;
    border-radius: 3px;
    background: linear-gradient(to right, var(--faint) 1px, transparent 1px) 0 100% / 3px 1px repeat-x;
    transition: background-color var(--dur-ui) var(--ease-ui);
  }
  .tex :global([data-sym]:focus-visible), .math-block :global([data-sym]:focus-visible) {
    outline: 1px solid var(--riddle-line);
    outline-offset: 1px;
    background-color: var(--riddle-wash);
  }
  @media (hover: hover) and (pointer: fine) {
    .tex :global([data-sym]:hover), .math-block :global([data-sym]:hover) { background-color: var(--riddle-wash); }
  }
</style>
