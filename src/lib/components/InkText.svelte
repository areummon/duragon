<script lang="ts">
  import { highlight, parseReply, renderTex } from '../rich';

  interface Props { text: string; animate?: boolean }
  let { text, animate = false }: Props = $props();

  type Token = { kind: 'space' | 'word' | 'code' | 'math'; text: string; order: number };
  type Piece =
    | { kind: 'para'; tokens: Token[] }
    | { kind: 'code'; html: string; order: number }
    | { kind: 'math'; html: string; order: number };

  // Words ink in one by one; inline code, a formula and a whole block each ink in as one token.
  const pieces = $derived.by(() => {
    let order = 0;
    return parseReply(text).map((b): Piece => {
      if (b.kind === 'code') return { kind: 'code', html: highlight(b.code, b.lang), order: order++ };
      if (b.kind === 'math') return { kind: 'math', html: renderTex(b.tex, true), order: order++ };
      const tokens: Token[] = [];
      for (const x of b.inlines) {
        if (x.kind === 'code') tokens.push({ kind: 'code', text: x.text, order: order++ });
        else if (x.kind === 'math') tokens.push({ kind: 'math', text: renderTex(x.text, false), order: order++ });
        else for (const w of x.text.split(/(\s+)/)) {
          if (!w) continue;
          tokens.push(/^\s+$/.test(w) ? { kind: 'space', text: w, order } : { kind: 'word', text: w, order: order++ });
        }
      }
      return { kind: 'para', tokens };
    });
  });

  const delay = (order: number) => (animate ? `calc(var(--delay-ink-word) * ${order})` : null);
</script>

<!-- Spans throughout, display:block where needed, so a reply stays valid inside the <p>, <span> and <button> that hold it. -->
{#each pieces as p, i (i)}{#if p.kind === 'para'}<span class="para"
      >{#each p.tokens as t, j (j)}{#if t.kind === 'space'}{t.text}{:else if t.kind === 'code'}<code
            class:w={animate}
            style:animation-delay={delay(t.order)}>{t.text}</code
          >{:else if t.kind === 'math'}<span class="tex" class:w={animate} style:animation-delay={delay(t.order)}
            >{@html t.text}</span
          >{:else if animate}<span class="w" style:animation-delay={delay(t.order)}>{t.text}</span
          >{:else}{t.text}{/if}{/each}</span
    >{:else if p.kind === 'code'}<span class="block code-block" class:in={animate} style:animation-delay={delay(p.order)}
      ><code>{@html p.html}</code></span
    >{:else}<span class="block math-block" class:in={animate} style:animation-delay={delay(p.order)}
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
</style>
