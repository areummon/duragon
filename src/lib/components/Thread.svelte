<script lang="ts">
  import { Ellipsis, Maximize2, Pin, Send, Volume2 } from 'lucide-svelte';
  import { readAloud, stopSpeaking, voice } from '../state/voice.svelte';
  import Sources from './Sources.svelte';
  import { tick } from 'svelte';
  import type { Message, Thread } from '../api/types';
  import { open as openThread, reply, setStatus, threads, togglePin } from '../state/threads.svelte';
  import { enlarge } from '../state/layout.svelte';
  import Blot from './Blot.svelte';
  import Drawing from './Drawing.svelte';
  import InkText from './InkText.svelte';
  import ThreadMenu, { type ThreadAction } from './ThreadMenu.svelte';

  interface Props {
    thread: Thread;
    side?: 'left' | 'right';
    /** margin: beside the page. inline: unfolded under its line. card: in the page when margins are marks. dialog: full screen. */
    variant?: 'margin' | 'inline' | 'dialog' | 'card';
  }
  let { thread, side = 'right', variant = 'margin' }: Props = $props();

  const inline = $derived(variant === 'inline');
  const isOpen = $derived(variant === 'dialog' || inline || threads.openId === thread.id);
  const first = $derived(thread.messages[0]);
  const replies = $derived(thread.messages.length - 1);
  const drawn = $derived(thread.messages.find((m) => m.drawing !== 'none'));
  const waiting = $derived(!!threads.replying[thread.id]);
  const fresh = $derived(thread.messages.some((m) => threads.freshText[m.id]));
  const actions = $derived<ThreadAction[]>(
    inline ? ['copy', 'speak', 'note', 'margin', 'dismiss']
    : variant === 'dialog' ? ['resolve', 'copy', 'speak', 'note', 'dismiss']
    : ['full', 'resolve', 'copy', 'speak', 'note', 'dismiss'],
  );

  let leaving = $state(false);
  let menu = $state(false);
  let draft = $state('');
  let scroller = $state<HTMLElement>();
  let field = $state<HTMLTextAreaElement>();
  let above = $state(false);
  let overflows = $state(false);

  const playingId = $derived(voice.playing ? voice.speakingId : null);

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
    if (!scroller) return;
    above = scroller.scrollTop > 8;
    overflows = scroller.scrollHeight > scroller.clientHeight + 1;
  }

  async function toEnd(smooth: boolean) {
    await tick();
    scroller?.scrollTo({ top: scroller.scrollHeight, behavior: smooth ? 'smooth' : 'instant' });
    onscroll();
  }

  /** Opening lands on the latest message; each new one scrolls into view. */
  let seen = 0;
  $effect(() => {
    const n = thread.messages.length + (waiting ? 1 : 0);
    if (!isOpen || !scroller) {
      seen = 0;
      return;
    }
    void toEnd(seen > 0);
    seen = n;
  });

  $effect(() => {
    if (inline && field) field.focus({ preventScroll: true });
  });

  /** The reply box grows with what you write. */
  $effect(() => {
    void draft;
    if (!field) return;
    field.style.height = 'auto';
    field.style.height = `${field.scrollHeight}px`;
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

  /** A press anywhere outside an open, unpinned thread folds it. */
  function outside(e: PointerEvent) {
    if (variant === 'dialog' || variant === 'card' || threads.openId !== thread.id || thread.pinned || threads.fullId) return;
    const t = e.target as Element | null;
    if (!t || article?.contains(t) || t.closest(`[aria-modal="true"], [data-thread="${thread.id}"]`)) return;
    openThread(null);
  }

  function speak(m: Message) {
    if (playingId === m.id) void stopSpeaking();
    else void readAloud(m.id, m.text);
  }

  function writeKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      send();
    }
  }
</script>

<svelte:window onclick={() => (menu = false)} onpointerdown={outside} />

<article
  class="thread {side} {variant}"
  class:open={isOpen}
  class:leaving
  data-thread-id={thread.id}
  role={inline ? 'dialog' : undefined}
  aria-label={inline ? 'Conversation with Duragon' : undefined}
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
      {#if overflows}
        <div class="earlier-row">
          <button class="earlier" class:shown={above} tabindex={above ? 0 : -1} onclick={() => scroller?.scrollTo({ top: 0, behavior: 'smooth' })}>↑ Earlier</button>
        </div>
      {/if}
      <div class="scroll" class:faded={above} bind:this={scroller} {onscroll}>
        {#each thread.messages as m, i (m.id)}
          <div class="msg {m.author}">
            {#if m.author === 'you'}
              <p>{m.text}</p>
            {:else}
              <p>
                {#if inline && i === 0}
                  <span class="close" role="button" tabindex="0" aria-label="Close this thread"
                    onclick={() => openThread(null)}
                    onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && openThread(null)}
                  ><InkText text={m.text} animate={!!threads.freshText[m.id]} /></span>
                {:else}
                  <InkText text={m.text} animate={!!threads.freshText[m.id] || variant === 'card'} />
                {/if}<button class="speak" aria-label={playingId === m.id ? 'Stop' : 'Read aloud'} title={playingId === m.id ? 'Stop' : 'Read aloud'} onclick={() => speak(m)}>{#if playingId === m.id}<span class="stop"></span>{:else}<Volume2 size={12} strokeWidth={1.7} />{/if}</button>
              </p>
              <Sources sources={m.sources} />
              {#if m.drawing !== 'none'}
                <div class="art"><Drawing status={m.drawing} svg={m.svg} fresh={!!threads.freshDraw[m.id]} size={variant === 'dialog' ? 'dialog' : variant === 'margin' ? 'margin' : 'large'} onLarger={() => enlarge(m.svg)} /></div>
              {/if}
            {/if}
          </div>
        {/each}
        {#if waiting}<div class="thinking"><Blot size={15} seed="thinking" waiting /></div>{/if}
      </div>

      <form class="write" onsubmit={(e) => { e.preventDefault(); send(); }}>
        <textarea
          rows="1"
          maxlength="4000"
          placeholder="Write back…"
          aria-label="Reply to Duragon"
          bind:value={draft}
          bind:this={field}
          onkeydown={writeKey}
        ></textarea>
        <button type="submit" class="send" aria-label="Send" disabled={!draft.trim()}><Send size={15} strokeWidth={1.6} /></button>
      </form>

      <div class="controls">
        {#if !inline}<button class="text" onclick={() => leave('resolved')}>Resolve</button>{/if}
        <button class="text" onclick={() => leave('dismissed')}>Dismiss</button>
        <span class="grow"></span>
        <button class="icon" aria-pressed={thread.pinned} aria-label="Pin open while you write" title="Pin open while you write" onclick={() => togglePin(thread.id)}><Pin size={14} strokeWidth={1.6} /></button>
        {#if variant !== 'dialog'}
          <button class="icon" aria-label="Open full screen" title="Open full screen" onclick={() => (threads.fullId = thread.id)}><Maximize2 size={14} strokeWidth={1.6} /></button>
        {/if}
        <span class="more">
          <button class="icon" aria-label="Thread options" aria-haspopup="menu" aria-expanded={menu} onclick={(e) => { e.stopPropagation(); menu = !menu; }}><Ellipsis size={16} strokeWidth={1.6} /></button>
          {#if menu}<ThreadMenu {thread} {actions} onclose={() => (menu = false)} onleave={leave} />{/if}
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
    stroke: var(--riddle);
    stroke-opacity: 0.32;
    stroke-width: 1.5;
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
  .earlier-row { display: flex; justify-content: center; height: 24px; margin-bottom: 6px; }
  .earlier {
    padding: 2px 10px;
    border-radius: 999px;
    box-shadow: 0 0 0 1px var(--rule);
    font-family: var(--font-sans);
    font-size: 12px;
    line-height: 1.3;
    color: var(--faint);
    opacity: 0;
    pointer-events: none;
    transition: opacity 200ms var(--ease-ui), color 200ms var(--ease-ui);
  }
  .earlier.shown { opacity: 1; pointer-events: auto; }
  .earlier:hover { color: var(--ink); }
  .scroll {
    position: relative;
    max-height: 60vh;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--rule) transparent;
    scrollbar-gutter: stable;
    text-align: left;
    padding-right: 6px;
  }
  .scroll.faded {
    mask-image: linear-gradient(transparent 0, #000 40px);
    -webkit-mask-image: linear-gradient(transparent 0, #000 40px);
  }
  .dialog .scroll { max-height: none; }
  .msg + .msg { margin-top: 16px; }
  .msg.you p { font-family: var(--font-sans); font-size: 15px; line-height: 1.55; color: var(--graphite); white-space: pre-wrap; }
  .close { cursor: pointer; }
  .close:focus-visible { outline: none; text-decoration: underline dotted var(--riddle-line); }
  .speak {
    display: inline-grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin-left: 8px;
    vertical-align: -3px;
    border-radius: 999px;
    color: var(--faint);
    transition: color 150ms var(--ease-ui);
  }
  .speak:hover { color: var(--ink); }
  .thinking { display: flex; align-items: center; height: 1lh; margin-top: 12px; }

  .write {
    position: relative;
    display: flex;
    align-items: flex-end;
    gap: 8px;
    margin-top: 16px;
    animation: reply-in var(--dur-reply-in) var(--ease-settle) both;
  }
  .write::after {
    content: '';
    position: absolute;
    inset: auto 0 0 0;
    height: 1.5px;
    border-radius: 1px;
    background: linear-gradient(to right, var(--riddle) 0%, color-mix(in oklab, var(--riddle) 55%, transparent) 62%, transparent 100%);
    opacity: 0;
    transform: scaleX(0);
    transform-origin: left center;
    pointer-events: none;
    transition: transform var(--dur-reply-underline) var(--ease-reply), opacity 180ms ease-out;
  }
  .write:focus-within::after { opacity: 1; transform: scaleX(1); }
  .write textarea {
    flex: 1;
    min-width: 0;
    padding: 0 0 6px;
    border: 0;
    resize: none;
    overflow: hidden;
    background: transparent;
    outline: none;
    font-family: var(--font-sans);
    font-size: 15px;
    line-height: 1.55;
    color: var(--ink);
    caret-color: var(--ink);
    text-align: left;
  }
  .write textarea::placeholder { color: color-mix(in srgb, var(--faint) 70%, transparent); }
  .send { flex: none; margin-bottom: 8px; color: var(--riddle); opacity: 1; transition: opacity 150ms var(--ease-ui); }
  .send:hover:not(:disabled) { opacity: 0.7; }
  .send:disabled { opacity: 0.25; cursor: default; }

  .controls {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-top: 10px;
    font-family: var(--font-sans);
    font-size: 12.5px;
    line-height: 1;
    color: var(--faint);
    text-align: left;
  }
  .grow { flex: 1; }
  .controls .text { transition: color 150ms var(--ease-ui); }
  .controls .text:hover { color: var(--ink); }
  .icon {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    margin-right: -12px;
    border-radius: 999px;
    color: var(--faint);
    transition: color 150ms var(--ease-ui), background-color 150ms var(--ease-ui);
  }
  .icon:hover { color: var(--ink); background: color-mix(in srgb, var(--rule) 50%, transparent); }
  .icon[aria-pressed='true'] { color: var(--ink); }
  .icon :global(.lucide-pin) { rotate: 45deg; transition: rotate 150ms var(--ease-ui); }
  .icon[aria-pressed='true'] :global(.lucide-pin) { rotate: 0deg; }
  .more { position: relative; }

  .stop { width: 8px; height: 8px; border-radius: 1.5px; background: currentColor; }
  .thread.card {
    margin: 14px 0 18px;
    padding: 18px 20px 14px;
    border: 1px solid var(--rule);
    border-radius: 14px;
    font-size: calc(var(--note-size) * 1.06);
    line-height: 1.55;
  }
  .thread.inline {
    font-size: inherit;
    line-height: inherit;
    animation: unfold var(--dur-unfold) var(--ease-step) backwards;
  }
  .inline .scroll { max-height: min(52vh, 520px); padding-right: 14px; }
  .inline .msg + .msg { margin-top: 12px; }
  .thread.dialog { padding: 0; animation: none; font-size: 17px; }
  .thread.dialog .art { margin-top: 18px; }
</style>
