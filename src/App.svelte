<script lang="ts">
  import { onMount } from 'svelte';
  import ChatPanel from './lib/components/ChatPanel.svelte';
  import LazyDialog from './lib/components/LazyDialog.svelte';
  import Page from './lib/components/Page.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import TopBar from './lib/components/TopBar.svelte';
  import ListeningPill from './lib/components/ListeningPill.svelte';
  import Toast from './lib/components/Toast.svelte';
  import { initVoice, stopHandsFree, voice, voiceBlur, voiceKeydown, voiceKeyup } from './lib/state/voice.svelte';
  import { closeTop } from './lib/escape';
  import { cycleTab, topDialog } from './lib/focus';
  import { isMod } from './lib/keys';
  import { enlarge, lightbox } from './lib/state/layout.svelte';
  import { threads } from './lib/state/threads.svelte';
  import {
    app, copyNote, exportNote, initApp, newNote, toggleChat, toggleJustMyWords,
  } from './lib/state/app.svelte';

  onMount(() => {
    void initApp().then(initVoice);
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
    const ctrl = e.ctrlKey && !e.metaKey;
    const mod = isMod(e);
    const handled = (fn: () => void) => {
      e.preventDefault();
      fn();
    };
    // A shortcut opens or closes nothing beneath a dialog, out of sight. A dialog on top closes by its own.
    const toggle = (onTop: boolean, act: () => void) =>
      handled(() => {
        if (onTop || !topDialog()) act();
      });
    if (mod && !e.shiftKey && e.key.toLowerCase() === 'k') toggle(app.paletteOpen, () => (app.paletteOpen = !app.paletteOpen));
    else if (mod && !e.shiftKey && e.key.toLowerCase() === 'j') toggle(false, toggleChat);
    else if (mod && !e.shiftKey && e.key === ',') toggle(app.customizeOpen && !app.mindEditor, () => (app.customizeOpen = !app.customizeOpen));
    else if (mod && e.shiftKey && e.code === 'KeyC') handled(() => void copyNote());
    else if (mod && e.shiftKey && e.code === 'KeyE') handled(() => void exportNote('md'));
    else if (mod && e.shiftKey && e.code === 'Period') handled(() => void toggleJustMyWords());
    else if (ctrl && !e.shiftKey && e.key.toLowerCase() === 'n') handled(() => void newNote());
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
  <LazyDialog open={app.customizeOpen} close={() => (app.customizeOpen = false)} load={() => import('./lib/components/customize/CustomizeDialog.svelte')} />
  <LazyDialog open={app.mindEditor !== null} close={() => (app.mindEditor = null)} load={() => import('./lib/components/customize/MindDialog.svelte')} />
  <Toast />
{/if}
