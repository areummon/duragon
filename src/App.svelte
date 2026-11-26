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
  import { threads } from './lib/state/threads.svelte';
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
