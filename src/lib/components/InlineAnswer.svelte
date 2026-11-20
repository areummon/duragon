<script lang="ts">
  import { Ellipsis } from 'lucide-svelte';
  import { enlarge } from '../state/layout.svelte';
  import { find, open, threads } from '../state/threads.svelte';
  import Blot from './Blot.svelte';
  import Drawing from './Drawing.svelte';
  import InkText from './InkText.svelte';
  import Sources from './Sources.svelte';
  import Thread from './Thread.svelte';
  import ThreadMenu, { type ThreadAction } from './ThreadMenu.svelte';

  let { id }: { id: string } = $props();
  const thread = $derived(find(id));
  const m = $derived(thread?.messages.find((x) => x.author === 'riddled'));
  const replies = $derived((thread?.messages.length ?? 1) - 1);
  const isOpen = $derived(threads.openId === id);
  const fresh = $derived(!!thread && thread.messages.some((x) => threads.freshText[x.id]));
  const thinking = $derived(threads.thinking.some((t) => t.task === 'answer' && t.target === thread?.quote));
  const ACTIONS: ThreadAction[] = ['copy', 'speak', 'note', 'margin', 'dismiss'];
  let menu = $state(false);
</script>

{#if thread}
  <div class="answer" data-inline-answer class:open={isOpen}>
    <span class="blot"><Blot size={17} seed={id} landing={fresh} /></span>
    {#if isOpen}
      <Thread {thread} variant="inline" />
    {:else if m}
      <button class="text" onclick={() => open(id)}>
        <InkText text={m.text} animate={!!threads.freshText[m.id]} />
        {#if replies > 0}<span class="replies">{replies} {replies === 1 ? 'reply' : 'replies'}</span>{/if}
      </button>
      <Sources sources={m.sources} />
      {#if m.drawing !== 'none'}
        <div class="art"><Drawing status={m.drawing} svg={m.svg} fresh={!!threads.freshDraw[m.id]} size="large" onLarger={() => enlarge(m.svg)} /></div>
      {/if}
      <span class="options" class:shown={menu}>
        <button class="icon" aria-label="Thread options" aria-haspopup="menu" aria-expanded={menu} onclick={(e) => { e.stopPropagation(); menu = !menu; }}><Ellipsis size={16} strokeWidth={1.6} /></button>
        {#if menu}<ThreadMenu {thread} actions={ACTIONS} onclose={() => (menu = false)} />{/if}
      </span>
    {:else if thinking}
      <Blot size={15} seed="thinking" waiting />
    {/if}
  </div>
{/if}

<svelte:window onclick={() => (menu = false)} />

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
  .blot { position: absolute; left: -2px; top: calc((1lh - 17px) / 2); display: flex; }
  .text { display: block; width: 100%; padding-right: 32px; text-align: left; cursor: var(--cursor-pointer); white-space: pre-wrap; }
  .replies {
    display: block;
    margin-top: 6px;
    font-family: var(--font-sans);
    font-size: 12.5px;
    line-height: 1.4;
    color: var(--faint);
    transition: color var(--dur-ui) var(--ease-ui);
  }
  .answer:hover .replies { color: var(--graphite); }
  .art { margin-top: 18px; display: flex; justify-content: center; }
  .options {
    position: absolute;
    top: -1px;
    right: 0;
    opacity: 0;
    transition: opacity 150ms var(--ease-ui);
  }
  .answer:hover .options, .options:focus-within, .options.shown { opacity: 1; }
  .icon {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 999px;
    color: var(--faint);
    transition: color 150ms var(--ease-ui), background-color 150ms var(--ease-ui);
  }
  .icon:hover { color: var(--ink); background: color-mix(in srgb, var(--rule) 50%, transparent); }
  @media (pointer: coarse) { .options { opacity: 0.6; } }
</style>
