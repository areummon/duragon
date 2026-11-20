<script lang="ts">
  import { Menu, X } from 'lucide-svelte';
  import { app, toggleChat, toggleSidebar } from '../state/app.svelte';
  import { kbd } from '../keys';
  import { layout } from '../state/layout.svelte';
  import Blot from './Blot.svelte';
</script>

<header class="bar" class:scrolled={layout.scrolled}>
  <div class="brand">
  <button class="hamburger" aria-label={app.sidebarOpen ? 'Close notes' : 'Open notes'} aria-expanded={app.sidebarOpen} onclick={toggleSidebar}>
    <span class="swap" class:on={!app.sidebarOpen}><Menu size={18} strokeWidth={1.6} /></span>
    <span class="swap" class:on={app.sidebarOpen}><X size={18} strokeWidth={1.6} /></span>
  </button>
  <span class="mark"><Blot size={14} seed="duragon" /></span>
  <span class="wordmark">duragon.</span>
  </div>
  <button class="chat" aria-pressed={app.chatOpen} title={kbd('J')} onclick={toggleChat}>
    <Blot size={13} />
    Chat
  </button>
</header>

<style>
  .bar {
    position: fixed;
    inset: 0 0 auto 0;
    z-index: 20;
    height: 72px;
    display: flex;
    align-items: flex-start;
    gap: 7px;
    padding: 20px 23px 0 24px;
    pointer-events: none;
  }
  .bar > * { pointer-events: auto; }
  .brand {
    display: flex;
    align-items: flex-start;
    gap: 7px;
    height: 32px;
    margin: 0 0 0 -1px;
    padding: 0 0 0 1px;
    border: 1px solid transparent;
    border-radius: 999px;
    background: transparent;
    transition: border-color 200ms var(--ease-ui), background-color 200ms var(--ease-ui), padding 200ms var(--ease-ui);
  }
  .scrolled .brand { border-color: var(--rule); background: var(--paper); padding-right: 16px; }
  .scrolled .hamburger { margin-left: 4px; }
  .hamburger {
    position: relative;
    width: 32px;
    height: 32px;
    margin-top: 0;
    margin-right: 2px;
    color: var(--graphite);
    transition: opacity var(--dur-ui) var(--ease-ui);
  }
  .hamburger:hover { opacity: 0.7; }
  .swap {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    opacity: 0;
    transform: rotate(-45deg) scale(0.8);
    transition: opacity 200ms var(--ease-ui), transform 280ms var(--ease-step);
  }
  .swap.on { opacity: 1; transform: none; }
  .mark { margin-top: 8px; display: flex; }
  .wordmark {
    line-height: 32px !important;
    font-family: var(--font-voice);
    font-size: 17px;
    line-height: 1;
    letter-spacing: 0.005em;
    margin-left: 3px;
  }
  .chat {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 14px;
    border: 1px solid var(--rule);
    border-radius: 999px;
    font-size: 14px;
    color: var(--ink);
    transition: opacity var(--dur-ui) var(--ease-ui), background-color var(--dur-ui) var(--ease-ui);
  }
  .chat:hover { opacity: 0.75; }
  .chat[aria-pressed='true'] { background: var(--riddle-wash); }
</style>
