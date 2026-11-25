<script lang="ts">
  import ArrowUp from 'lucide-svelte/icons/arrow-up';
  import Pin from 'lucide-svelte/icons/pin';
  import X from 'lucide-svelte/icons/x';
  import { tick } from 'svelte';
  import { closeOnEscape } from '../escape';
  import { app } from '../state/app.svelte';
  import { chat, chatAsText, clearChat, loadChat, send, unloadChat } from '../state/chat.svelte';
  import { enlarge } from '../state/layout.svelte';
  import { riddlerName } from '../state/riddler';
  import { threads } from '../state/threads.svelte';
  import Blot from './Blot.svelte';
  import Drawing from './Drawing.svelte';
  import EarlierInChat from './EarlierInChat.svelte';
  import InkText from './InkText.svelte';
  import TrimmedHint from './TrimmedHint.svelte';
  import { askAboutSymbol } from '../symbolTip';

  const CHIPS = ['What am I still misunderstanding?', 'What have we figured out so far?', 'What should I work on next?'];

  let draft = $state('');
  let input = $state<HTMLInputElement>();
  let scroller = $state<HTMLElement>();
  let copied = $state(false);
  let copiedTimer: number | undefined;

  async function copyChat() {
    try {
      await navigator.clipboard.writeText(chatAsText());
    } catch {
      return;
    }
    copied = true;
    window.clearTimeout(copiedTimer);
    copiedTimer = window.setTimeout(() => (copied = false), 1200);
  }

  const messages = $derived(chat.messages);
  const open = $derived(app.chatOpen);
  $effect(() => {
    if (open) return closeOnEscape(() => (app.chatOpen = false));
  });
  const dim = $derived(open && !chat.pinned);
  const nThreads = $derived(threads.list.length);
  const nOpen = $derived(threads.list.filter((t) => t.status === 'open').length);

  $effect(() => {
    const id = app.note?.id;
    if (!id) {
      unloadChat();
      return;
    }
    if (open) {
      if (chat.noteId !== id) void loadChat(id);
    } else if (chat.noteId !== id) {
      unloadChat();
    }
  });

  $effect(() => {
    if (open) focusInput();
  });

  $effect(() => {
    void messages.length;
    void chat.waiting;
    void tick().then(() => scroller?.scrollTo({ top: scroller.scrollHeight, behavior: 'smooth' }));
  });

  /** Clear and the suggestions leave with their own button, so typing goes on in the field. */
  const focusInput = () => input?.focus({ preventScroll: true });

  function submit(text = draft) {
    if (!text.trim()) return;
    draft = '';
    void send(text);
    focusInput();
  }

  function clear() {
    void clearChat();
    focusInput();
  }

</script>

{#if dim}
  <div class="scrim" role="presentation" onclick={() => (app.chatOpen = false)}></div>
{/if}

<aside class="panel" class:open aria-label="Chat" inert={!open}>
  <header>
    <div class="who">
      <span class="name"><Blot size={13} seed="duragon" />{riddlerName()}</span>
      <span class="sub">On “{app.note?.title || 'Untitled'}”. Reading this note: {nThreads} {nThreads === 1 ? 'thread' : 'threads'} ({nOpen} open)</span>
    </div>
    <div class="tools">
      {#if messages.length}
        <button class="t" class:done={copied} onclick={copyChat}>{copied ? 'Copied' : 'Copy'}</button>
        <button class="t" onclick={clear}>Clear</button>
      {/if}
      <button class="i" aria-pressed={chat.pinned} aria-label="Pin open while you write" title="Pin open while you write" onclick={() => (chat.pinned = !chat.pinned)}><Pin size={15} strokeWidth={1.5} /></button>
      <button class="i" aria-label="Close" onclick={() => (app.chatOpen = false)}><X size={17} strokeWidth={1.5} /></button>
    </div>
  </header>

  <div class="scroll" bind:this={scroller}>
    <EarlierInChat />
    {#if messages.length === 0}
      <p class="empty">I've read the whole note and its threads. Ask me anything about it.</p>
      <div class="chips">
        {#each CHIPS as c (c)}<button onclick={() => submit(c)}>{c}</button>{/each}
      </div>
    {:else}
      {#each messages as m (m.id)}
        {#if m.author === 'you'}
          <div class="you" class:in={!!chat.fresh[m.id]}><InkText text={m.text} /></div>
        {:else}
          <div class="riddled">
            <InkText text={m.text} animate={!!chat.fresh[m.id]} symbols={m.symbols} onask={(s) => void send(askAboutSymbol(s))} />
            {#if m.trimmed}<TrimmedHint around="cursor" />{/if}
            {#if m.drawing !== 'none'}
              <div class="art"><Drawing status={m.drawing} svg={m.svg} fresh={!!chat.fresh[`d${m.id}`]} size="large" onLarger={() => enlarge(m.svg)} /></div>
            {/if}
          </div>
        {/if}
      {/each}
      {#if chat.waiting}<div class="wait"><Blot size={15} seed="thinking" waiting /></div>{/if}
    {/if}
  </div>

  <form class="compose" onsubmit={(e) => { e.preventDefault(); submit(); }}>
    <input bind:this={input} bind:value={draft} placeholder="Ask about this note…" aria-label="Ask about this note" />
    <button class="go" aria-label="Send" disabled={!draft.trim()}><ArrowUp size={16} strokeWidth={1.8} /></button>
  </form>
</aside>

<style>
  .scrim { position: fixed; inset: 0; z-index: 38; }
  .panel {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 40;
    display: flex;
    flex-direction: column;
    width: min(480px, 100vw);
    background: var(--paper);
    border-left: 1px solid var(--rule);
    transform: translateX(100%);
    opacity: 0;
    visibility: hidden;
    transition: transform var(--dur-drawer) var(--ease-drawer), opacity var(--dur-drawer) var(--ease-drawer), visibility 0s linear var(--dur-drawer);
  }
  .panel.open { transform: none; opacity: 1; visibility: visible; transition-delay: 0s; }

  header { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; padding: 20px 20px 10px 24px; }
  .who { display: flex; flex-direction: column; gap: 6px; min-width: 0; }
  .name { display: flex; align-items: center; gap: 9px; font-family: var(--font-voice); font-size: 17px; line-height: 1.2; color: var(--riddle); }
  .sub { font-size: 12.5px; line-height: 1.45; color: var(--faint); padding-right: 8px; }
  .tools { display: flex; align-items: center; gap: 4px; color: var(--faint); }
  .t { padding: 4px 8px; font-size: 13px; line-height: 1.2; transition: color var(--dur-ui) var(--ease-ui); }
  .t:hover, .i:hover, .i[aria-pressed='true'] { color: var(--ink); }
  .t.done { color: var(--ink); }
  .i { display: grid; place-items: center; width: 28px; height: 28px; transition: color var(--dur-ui) var(--ease-ui), transform 160ms var(--ease-step); }

  .scroll { flex: 1; overflow-y: auto; overscroll-behavior: contain; padding: 22px 24px 24px; display: flex; flex-direction: column; gap: 26px; scrollbar-width: thin; scrollbar-color: var(--rule) transparent; }
  .empty { font-family: var(--font-voice); font-size: 16.5px; line-height: 1.55; color: var(--riddle); }
  .chips { display: flex; flex-direction: column; align-items: flex-start; gap: 8px; margin-top: -8px; }
  .chips button {
    padding: 8px 14px;
    border: 1px solid var(--rule);
    border-radius: 999px;
    font-size: 13.5px;
    line-height: 1.3;
    color: var(--graphite);
    transition: color var(--dur-ui) var(--ease-ui), background-color var(--dur-ui) var(--ease-ui);
  }
  .chips button:hover { color: var(--ink); background: var(--riddle-wash); }

  .you { align-self: flex-end; max-width: 85%; padding: 8px 14px; border-radius: 16px; background: var(--shelf); font-size: 14px; line-height: 1.5; }
  .you.in { animation: ink-in var(--dur-ink-in) var(--ease-step) both; }
  .riddled { font-family: var(--font-voice); font-size: 16.5px; line-height: 1.55; color: var(--riddle); --ink-para-gap: 1.1em; }
  .art { margin-top: 18px; }
  .wait { display: flex; }

  .compose { position: relative; margin: 0; padding: 14px 20px 18px 24px; border-top: 1px solid var(--rule); }
  .compose input { width: 100%; padding: 8px 44px 8px 0; border: 0; outline: none; background: transparent; font-size: 15px; color: var(--ink); }
  .compose input::placeholder { color: var(--faint); }
  .go {
    position: absolute;
    right: 20px;
    top: 50%;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    margin-top: -16px;
    border-radius: 50%;
    background: var(--ink);
    color: var(--paper);
    transition: opacity var(--dur-ui) var(--ease-ui), transform 160ms var(--ease-step);
  }
  .go:disabled { opacity: 0.3; cursor: default; }
  .i:active, .go:active:not(:disabled) { transform: scale(0.97); }

  @media (prefers-reduced-motion: reduce) {
    .panel { transition: opacity var(--dur-drawer) var(--ease-drawer), visibility 0s linear var(--dur-drawer); }
    .panel.open { transition-delay: 0s; }
    .i:active, .go:active:not(:disabled) { transform: none; }
  }
</style>
