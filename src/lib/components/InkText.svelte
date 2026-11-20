<script lang="ts">
  interface Props { text: string; animate?: boolean }
  let { text, animate = false }: Props = $props();

  type Token = { kind: 'space' | 'word' | 'code'; text: string; order: number };

  // Inline `code` from the model renders as code; everything else is words that ink in one by one.
  const tokens = $derived.by(() => {
    const out: Token[] = [];
    let order = 0;
    for (const part of text.split(/(`[^`\n]+`)/)) {
      if (!part) continue;
      if (part.length > 2 && part.startsWith('`') && part.endsWith('`')) {
        out.push({ kind: 'code', text: part.slice(1, -1), order: order++ });
        continue;
      }
      for (const w of part.split(/(\s+)/)) {
        if (!w) continue;
        out.push(/^\s+$/.test(w) ? { kind: 'space', text: w, order } : { kind: 'word', text: w, order: order++ });
      }
    }
    return out;
  });
</script>

{#each tokens as t, i (i)}{#if t.kind === 'space'}{t.text}{:else if t.kind === 'code'}<code
      class:w={animate}
      style:animation-delay={animate ? `calc(var(--delay-ink-word) * ${t.order})` : null}>{t.text}</code
    >{:else if animate}<span class="w" style:animation-delay="calc(var(--delay-ink-word) * {t.order})">{t.text}</span
    >{:else}{t.text}{/if}{/each}

<style>
  .w { animation: ink-word var(--dur-ink-word) var(--ease-ink) both; }
  code {
    font-family: ui-monospace, 'JetBrains Mono', monospace;
    font-size: 0.86em;
    padding: 0.08em 0.32em;
    border-radius: 4px;
    background: var(--code);
  }
</style>
