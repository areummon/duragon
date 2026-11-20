<script lang="ts">
  import { ArrowUp, Ellipsis, Maximize2, Pin, Send, Volume2 } from 'lucide-svelte';
  import { readAloud, stopSpeaking, voice } from '../state/voice.svelte';
  import Sources from './Sources.svelte';
  import { tick } from 'svelte';
  import type { Thread } from '../api/types';
  import {
    makeNote, open as openThread, reply, setStatus, threads, threadText, togglePin,
  } from '../state/threads.svelte';
  import { enlarge } from '../state/layout.svelte';
  import Blot from './Blot.svelte';
  import Drawing from './Drawing.svelte';
  import InkText from './InkText.svelte';

  interface Props {
    thread: Thread;
    side?: 'left' | 'right';
    variant?: 'margin' | 'dialog' | 'card';
  }
  let { thread, side = 'right', variant = 'margin' }: Props = $props();

  const isOpen = $derived(variant === 'dialog' || threads.openId === thread.id);
  const first = $derived(thread.messages[0]);
  const replies = $derived(thread.messages.length - 1);
  const drawn = $derived(thread.messages.find((m) => m.drawing !== 'none'));
  const lastRiddled = $derived([...thread.messages].reverse().find((m) => m.author === 'riddled'));
  const playing = $derived(voice.playing && !!lastRiddled && voice.speakingId === lastRiddled.id);
  const waiting = $derived(!!threads.replying[thread.id]);
  const fresh = $derived(thread.messages.some((m) => threads.freshText[m.id]));

  let leaving = $state(false);
  let menu = $state(false);
  let draft = $state('');
  let scroller = $state<HTMLElement>();
  let above = $state(false);

  function leave(status: 'resolved' | 'dismissed') {
    menu = false;
    leaving = true;
    window.setTimeout(() => void setStatus(thread.id, status), 200);
  }

  function send() {
    const text = draft.trim();
    if (!text) return;
    draft = '';
    void reply(thread.id, text);
  }

  function onscroll() {
    above = (scroller?.scrollTop ?? 0) > 8;
  }

  async function toEnd() {
    await tick();
    scroller?.scrollTo({ top: scroller.scrollHeight, behavior: 'smooth' });
  }

  $effect(() => {
    void thread.messages.length;
    void waiting;
    if (isOpen && thread.messages.length > 1) void toEnd();
  });

  let article: HTMLElement | undefined = $state();

  $effect(() => {
    const el = article;
    if (!el) return;
    const enter = () => (threads.hoverId = thread.id);
    const leave = () => (threads.hoverId = null);
    el.addEventListener('keydown', keydown);
    el.addEventListener('mouseenter', enter);
    el.addEventListener('mouseleave', leave);
    return () => {
      el.removeEventListener('keydown', keydown);
      el.removeEventListener('mouseenter', enter);
      el.removeEventListener('mouseleave', leave);
    };
  });

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && variant !== 'dialog' && isOpen) {
      e.stopPropagation();
      openThread(null);
    }
  }

  const copy = () => {
    menu = false;
    void navigator.clipboard?.writeText(threadText(thread));
  };
  const fullscreen = () => {
    menu = false;
    threads.fullId = thread.id;
  };
  const speak = () => {
    menu = false;
    if (!lastRiddled) return;
    if (playing) void stopSpeaking();
    else void readAloud(lastRiddled.id, lastRiddled.text);
  };
</script>

<svelte:window onclick={() => (menu = false)} />

<article
  class="thread {side} {variant}"
  class:open={isOpen}
  class:leaving
  data-thread-id={thread.id}
  bind:this={article}
>
  {#if variant === 'margin'}
    <span class="blot-slot"><Blot size={17} seed={thread.id} landing={fresh} /></span>
    {#if isOpen}
      <svg class="edge" aria-hidden="true" preserveAspectRatio="none"><line x1="0.5" y1="0" x2="0.5" y2="100%" pathLength="1" /></svg>
    {/if}
  {/if}

  {#if !isOpen}
    <div class="collapsed">
      <button class="first" onclick={() => openThread(thread.id)}>
        {#if first}<InkText text={first.text} animate={!!threads.freshText[first.id]} />{/if}
        {#if replies > 0}<span class="replies">{replies} {replies === 1 ? 'reply' : 'replies'}</span>{/if}
      </button>
      {#if drawn}
        <div class="art"><Drawing status={drawn.drawing} svg={drawn.svg} fresh={!!threads.freshDraw[drawn.id]} onLarger={() => enlarge(drawn.svg)} /></div>
      {/if}
    </div>
  {:else}
    <div class="body">
      <div class="scroll" bind:this={scroller} {onscroll}>
        {#if above}
          <button class="earlier" onclick={() => scroller?.scrollTo({ top: 0, behavior: 'smooth' })}><ArrowUp size={11} strokeWidth={1.6} />Earlier</button>
        {/if}
        {#each thread.messages as m (m.id)}
          <div class="msg {m.author}">
            {#if m.author === 'you'}
              <p>{m.text}</p>
            {:else}
              <p><InkText text={m.text} animate={!!threads.freshText[m.id] || variant === 'card'} />{#if m === lastRiddled}<button class="speak" aria-label={playing ? 'Stop' : 'Read aloud'} title={playing ? 'Stop' : 'Read aloud'} onclick={speak}>{#if playing}<span class="stop"></span>{:else}<Volume2 size={13} strokeWidth={1.5} />{/if}</button>{/if}</p>
              <Sources sources={m.sources} />
              {#if m.drawing !== 'none'}
                <div class="art"><Drawing status={m.drawing} svg={m.svg} fresh={!!threads.freshDraw[m.id]} size={variant === 'dialog' ? 'dialog' : variant === 'card' ? 'large' : 'margin'} onLarger={() => enlarge(m.svg)} /></div>
              {/if}
            {/if}
          </div>
        {/each}
        {#if waiting}<div class="thinking"><Blot size={15} seed="thinking" waiting /></div>{/if}
      </div>

      <div class="write">
        <input
          placeholder="Write back…"
          aria-label="Write back"
          bind:value={draft}
          onkeydown={(e) => e.key === 'Enter' && !e.isComposing && send()}
        />
        <button class="send" aria-label="Send" disabled={!draft.trim()} onclick={send}><Send size={15} strokeWidth={1.4} /></button>
        <span class="underline"></span>
      </div>

      <div class="controls">
        <button class="text" onclick={() => leave('resolved')}>Resolve</button>
        <button class="text" onclick={() => leave('dismissed')}>Dismiss</button>
        <span class="grow"></span>
        <button class="icon" aria-pressed={thread.pinned} aria-label="Keep open while writing" title="Keep open while writing" onclick={() => togglePin(thread.id)}><Pin size={14} strokeWidth={1.5} /></button>
        {#if variant !== 'dialog'}
          <button class="icon" aria-label="Open full screen" title="Full screen" onclick={fullscreen}><Maximize2 size={14} strokeWidth={1.5} /></button>
        {/if}
        <span class="more">
          <button class="icon" aria-label="More" aria-expanded={menu} onclick={(e) => { e.stopPropagation(); menu = !menu; }}><Ellipsis size={15} strokeWidth={1.5} /></button>
          {#if menu}
            <div class="popover menu" role="menu">
              {#if variant !== 'dialog'}<button class="menu-item" role="menuitem" onclick={fullscreen}>Open full screen</button>{/if}
              <button class="menu-item" role="menuitem" onclick={() => leave('resolved')}>Resolve</button>
              <button class="menu-item" role="menuitem" onclick={copy}>Copy</button>
              <button class="menu-item" role="menuitem" onclick={speak}>{playing ? 'Stop reading' : 'Read aloud'}</button>
              <button class="menu-item" role="menuitem" onclick={() => void makeNote(thread.id)}>Make a note</button>
              <div class="menu-sep"></div>
              <button class="menu-item" role="menuitem" onclick={() => leave('dismissed')}>Dismiss</button>
            </div>
          {/if}
        </span>
      </div>
    </div>
  {/if}
</article>

<style>
  .thread {
    position: relative;
    font-family: var(--font-voice);
    font-size: 16px;
    line-height: 1.5;
    color: var(--riddle);
    animation: settle var(--dur-settle) var(--ease-settle) both;
    transition: opacity var(--dur-settle) var(--ease-settle), transform var(--dur-settle) var(--ease-settle);
  }
  .thread.leaving { opacity: 0; transform: translateY(4px); pointer-events: none; }
  .thread.margin.right { padding-left: 21px; text-align: left; }
  .thread.margin.left { padding-right: 21px; text-align: right; }
  .blot-slot { position: absolute; top: 4px; display: flex; }
  .right .blot-slot { left: -9px; }
  .left .blot-slot { right: -9px; }
  .edge {
    position: absolute;
    top: 26px;
    height: calc(100% - 26px);
    width: 1px;
    overflow: visible;
  }
  .right .edge { left: -1.5px; }
  .left .edge { right: -1.5px; }
  .edge line {
    stroke: var(--riddle-line);
    stroke-width: 1;
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    vector-effect: non-scaling-stroke;
    animation: tether-draw var(--dur-tether-2) var(--ease-tether) var(--delay-tether-2) both;
  }

  .first { display: block; width: 100%; text-align: inherit; cursor: pointer; }
  .replies { display: block; margin-top: 2px; font-family: var(--font-sans); font-size: 12.5px; line-height: 1.4; color: var(--faint); }
  .art { margin-top: 14px; }
  .left .art :global(figure) { margin-left: auto; }
  .left .art :global(.player) { text-align: left; }
  .left .art :global(.wait) { justify-content: flex-end; }

  .body { display: flex; flex-direction: column; }
  .scroll {
    position: relative;
    max-height: 60vh;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--rule) transparent;
    text-align: left;
    padding-right: 6px;
  }
  .dialog .scroll { max-height: none; }
  .earlier {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 4px;
    margin: 0 auto 8px;
    padding: 3px 10px 3px 8px;
    border: 1px solid var(--rule);
    border-radius: 999px;
    background: var(--paper);
    font-family: var(--font-sans);
    font-size: 12px;
    line-height: 1.3;
    color: var(--graphite);
    animation: ui-fade var(--dur-ui) var(--ease-ui) both;
  }
  .msg + .msg { margin-top: 16px; }
  .msg.you p { font-family: var(--font-sans); font-size: 15px; line-height: 1.5; color: var(--graphite); }
  .speak { display: inline-grid; place-items: center; width: 20px; height: 20px; margin-left: 8px; vertical-align: -3px; color: var(--faint); transition: color var(--dur-ui) var(--ease-ui); }
  .speak:hover { color: var(--ink); }
  .thinking { margin-top: 12px; display: flex; }

  .write {
    position: relative;
    display: flex;
    align-items: center;
    margin-top: 18px;
    animation: reply-in var(--dur-reply-in) var(--ease-settle) both;
  }
  .write input {
    flex: 1;
    min-width: 0;
    padding: 8px 28px 8px 0;
    border: 0;
    background: transparent;
    outline: none;
    font-family: var(--font-sans);
    font-size: 15px;
    color: var(--ink);
    text-align: left;
  }
  .write input::placeholder { color: var(--faint); }
  .send { position: absolute; right: 0; display: grid; place-items: center; width: 26px; height: 26px; color: var(--faint); transition: color var(--dur-ui) var(--ease-ui); }
  .send:not(:disabled) { color: var(--ink); }
  .send:disabled { cursor: default; }
  .underline {
    position: absolute;
    inset: auto 0 0 0;
    height: 1px;
    background: var(--rule);
    transform-origin: left;
    animation: reply-underline var(--dur-reply-underline) var(--ease-reply) both;
  }
  .write:focus-within .underline { background: var(--riddle-line); }

  .controls {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-top: 14px;
    font-family: var(--font-sans);
    font-size: 13px;
    line-height: 1;
    color: var(--faint);
    text-align: left;
  }
  .grow { flex: 1; }
  .controls button { transition: color var(--dur-ui) var(--ease-ui); }
  .controls button:hover { color: var(--ink); }
  .icon { display: grid; place-items: center; width: 24px; height: 24px; margin: 0 -4px; }
  .icon[aria-pressed='true'] { color: var(--ink); }
  .more { position: relative; }
  .menu { top: 100%; right: 0; margin-top: 4px; min-width: 170px; white-space: nowrap; }

  .stop { width: 8px; height: 8px; border-radius: 1.5px; background: currentColor; }
  .thread.card {
    margin: 14px 0 18px;
    padding: 18px 20px 14px;
    border: 1px solid var(--rule);
    border-radius: 14px;
    font-size: calc(var(--note-size) * 1.06);
    line-height: 1.55;
  }
  .thread.dialog { padding: 0; animation: none; font-size: 17px; }
  .thread.dialog .art { margin-top: 18px; }
</style>
