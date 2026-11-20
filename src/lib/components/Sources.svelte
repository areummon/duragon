<script lang="ts">
  import Globe from 'lucide-svelte/icons/globe';
  import type { Source } from '../api/types';

  let { sources }: { sources: Source[] } = $props();

  const host = (url: string) => {
    try {
      return new URL(url).hostname.replace(/^www\./, '');
    } catch {
      return url;
    }
  };
  const shown = $derived(sources.slice(0, 3));

  async function go(e: MouseEvent, url: string) {
    e.preventDefault();
    if ('__TAURI_INTERNALS__' in window) {
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      await openUrl(url);
    } else {
      window.open(url, '_blank', 'noopener');
    }
  }
</script>

{#if shown.length}
  <div class="sources">
    <Globe size={12} strokeWidth={1.5} />
    {#each shown as s, i (s.url)}
      <a href={s.url} title={s.title} target="_blank" rel="noopener noreferrer" onclick={(e) => go(e, s.url)}>{host(s.url)}</a>{#if i < shown.length - 1}<span class="dot">·</span>{/if}
    {/each}
  </div>
{/if}

<style>
  .sources {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 3px 6px;
    margin-top: 6px;
    font-family: var(--font-sans);
    font-size: 12.5px;
    line-height: 1.4;
    color: var(--faint);
    text-align: left;
  }
  .sources :global(svg) { flex: none; }
  a { color: inherit; text-decoration: none; transition: color var(--dur-ui) var(--ease-ui); }
  a:hover { color: var(--ink); text-decoration: underline; }
</style>
