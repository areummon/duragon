<script lang="ts">
  import { Volume2 } from 'lucide-svelte';
  import { enlarge } from '../state/layout.svelte';
  import { find, open, threads } from '../state/threads.svelte';
  import Blot from './Blot.svelte';
  import Drawing from './Drawing.svelte';
  import InkText from './InkText.svelte';
  import Sources from './Sources.svelte';

  let { id }: { id: string } = $props();
  const thread = $derived(find(id));
  const m = $derived(thread?.messages.find((x) => x.author === 'riddled'));
  const fresh = $derived(!!m && !!threads.freshText[m.id]);
  const thinking = $derived(threads.thinking.some((t) => t.task === 'answer' && t.target === thread?.quote));
</script>

{#if thread}
  <div class="answer" data-inline-answer class:active={threads.openId === id}>
    <span class="blot"><Blot size={17} seed={id} landing={fresh} /></span>
    {#if m}
      <button class="text" onclick={() => open(id)} title="Reply in the margin">
        <InkText text={m.text} animate={fresh} />
      </button>
      <Sources sources={m.sources} />
      {#if m.drawing !== 'none'}
        <div class="art"><Drawing status={m.drawing} svg={m.svg} fresh={!!threads.freshDraw[m.id]} size="large" onLarger={() => enlarge(m.svg)} /></div>
      {/if}
    {:else if thinking}
      <Blot size={15} seed="thinking" waiting />
    {/if}
  </div>
{/if}

<style>
  .answer {
    position: relative;
    margin: 18px 0;
    padding-left: 22px;
    font-family: var(--font-voice);
    font-size: calc(var(--note-size) * 1.06);
    line-height: 1.55;
    color: var(--riddle);
    letter-spacing: 0;
    animation: settle var(--dur-settle) var(--ease-settle) both;
  }
  .blot { position: absolute; left: -28px; top: 5px; display: flex; }
  .text { display: block; text-align: left; cursor: pointer; white-space: pre-wrap; }
  .art { margin-top: 18px; display: flex; justify-content: center; }
</style>
