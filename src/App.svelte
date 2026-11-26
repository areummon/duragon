<script lang="ts">
  import { onMount } from 'svelte';
  import ChatPanel from './lib/components/ChatPanel.svelte';
  import LazyDialog from './lib/components/LazyDialog.svelte';
  import Page from './lib/components/Page.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import TopBar from './lib/components/TopBar.svelte';
  import ListeningPill from './lib/components/ListeningPill.svelte';
  import Toast from './lib/components/Toast.svelte';
  import { loadCustomizeDialog, preloadCustomizeDialog } from './lib/components/customize/preload';
  import { initVoice, stopHandsFree, voice, voiceBlur, voiceKeydown, voiceKeyup } from './lib/state/voice.svelte';
  import { closeTop } from './lib/escape';
  import { cycleTab, topDialog } from './lib/focus';
  import { matchesAction } from './lib/state/keymap.svelte';
  import { enlarge, lightbox } from './lib/state/layout.svelte';
  import { settings } from './lib/state/settings.svelte';
  import { threads } from './lib/state/threads.svelte';
  import { vim } from './lib/editor/vim.svelte';
  import {
    app, copyNote, exportNote, initApp, newNote, toggleChat, toggleJustMyWords,
  } from './lib/state/app.svelte';

  onMount(() => {
    void initApp().then(() => {
      void initVoice();
      if ('requestIdleCallback' in window) window.requestIdleCallback(preloadCustomizeDialog);
      else setTimeout(preloadCustomizeDialog, 400);
    });
  });

  function onkeydown(e: KeyboardEvent) {
    voiceKeydown(e);
    cycleTab(e);
    // ProseMirror marks every Escape handled, so defaultPrevented can't tell; a field that uses Escape stops it first.
    if (e.key === 'Escape') {
      stopHandsFree();
      if (!e.isComposing) closeTop();
      return;
    }
    const handled = (fn: () => void) => {
      e.preventDefault();
      fn();
    };
    // A shortcut opens or closes nothing beneath a dialog, out of sight. A dialog on top closes by its own.
    const toggle = (onTop: boolean, act: () => void) =>
      handled(() => {
        if (onTop || !topDialog()) act();
      });
    if (matchesAction(e, 'find-note')) toggle(app.paletteOpen, () => (app.paletteOpen = !app.paletteOpen));
    else if (matchesAction(e, 'toggle-chat')) toggle(false, toggleChat);
    else if (matchesAction(e, 'open-customize')) toggle(app.customizeOpen && !app.mindEditor, () => (app.customizeOpen = !app.customizeOpen));
    else if (matchesAction(e, 'copy-note')) handled(() => void copyNote());
    else if (matchesAction(e, 'export-note')) handled(() => void exportNote('md'));
    else if (matchesAction(e, 'just-my-words')) handled(() => void toggleJustMyWords());
    else if (matchesAction(e, 'new-note')) toggle(false, () => void newNote());
  }
</script>

<svelte:window {onkeydown} onkeyup={voiceKeyup} onblur={voiceBlur} />

{#if app.ready}
  <TopBar />
  <Sidebar />
  <Page />
  {#if settings.value.vimMode}
    <div
      class="vim-pill"
      class:insert={vim.mode === 'insert'}
      role="status"
      aria-live="polite"
      data-testid="vim-mode-pill"
    >
      <span>{vim.mode.toUpperCase()}</span>
      {#if vim.pending}
        <span class="vim-pending">{vim.pending}</span>
      {/if}
    </div>
  {/if}
  {#if voice.mode === 'off'}
    <div class="hint">Hold Alt to speak · tap twice for hands-free</div>
  {:else}
    <ListeningPill />
  {/if}
  <ChatPanel />
  <LazyDialog open={threads.fullId !== null} close={() => (threads.fullId = null)} load={() => import('./lib/components/ThreadDialog.svelte')} />
  <LazyDialog open={lightbox.svg !== null} close={() => enlarge(null)} load={() => import('./lib/components/DrawingDialog.svelte')} />
  <LazyDialog open={app.trashOpen} close={() => (app.trashOpen = false)} load={() => import('./lib/components/TrashDialog.svelte')} />
  <LazyDialog open={app.paletteOpen} close={() => (app.paletteOpen = false)} load={() => import('./lib/components/CommandPalette.svelte')} remount />
  <LazyDialog open={app.customizeOpen} close={() => (app.customizeOpen = false)} load={loadCustomizeDialog} />
  <LazyDialog open={app.mindEditor !== null} close={() => (app.mindEditor = null)} load={() => import('./lib/components/customize/MindDialog.svelte')} />
  <Toast />
{/if}

<style>
  .vim-pill {
    position: fixed;
    left: 24px;
    bottom: 20px;
    z-index: 25;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 9px;
    border: 1px solid var(--rule);
    border-radius: 999px;
    background: var(--shelf);
    font-size: 11.5px;
    font-weight: 500;
    line-height: 1.3;
    letter-spacing: 0.04em;
    color: var(--ink);
    text-box: trim-both cap alphabetic;
    pointer-events: none;
    user-select: none;
  }
  .vim-pill.insert {
    color: var(--graphite);
  }
  .vim-pending {
    font-family: ui-monospace, 'SF Mono', Menlo, monospace;
    letter-spacing: 0;
    color: var(--faint);
  }
  @media print {
    .vim-pill {
      display: none;
    }
  }
</style>
