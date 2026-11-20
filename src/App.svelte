<script lang="ts">
  import { onMount } from 'svelte';
  import ChatPanel from './lib/components/ChatPanel.svelte';
  import DrawingDialog from './lib/components/DrawingDialog.svelte';
  import InkDrops from './lib/components/InkDrops.svelte';
  import ThreadDialog from './lib/components/ThreadDialog.svelte';
  import CommandPalette from './lib/components/CommandPalette.svelte';
  import Page from './lib/components/Page.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import TopBar from './lib/components/TopBar.svelte';
  import CustomizeDialog from './lib/components/customize/CustomizeDialog.svelte';
  import ListeningPill from './lib/components/ListeningPill.svelte';
  import MindDialog from './lib/components/customize/MindDialog.svelte';
  import Toast from './lib/components/Toast.svelte';
  import { initVoice, stopHandsFree, voice, voiceBlur, voiceKeydown, voiceKeyup } from './lib/state/voice.svelte';
  import { isMod } from './lib/keys';
  import { getBackend } from './lib/api/backend';
  import { scaleWheel } from './lib/motion/wheel';
  import {
    app, copyNote, exportNote, initApp, newNote, setSidebar, toggleChat, toggleJustMyWords,
  } from './lib/state/app.svelte';

  onMount(() => {
    void initApp().then(initVoice);
    void getBackend().then((b) => b.wheelScale()).then(scaleWheel);
  });

  function onkeydown(e: KeyboardEvent) {
    voiceKeydown(e);
    if (e.key === 'Escape') {
      stopHandsFree();
      if (!e.defaultPrevented && app.sidebarOpen && !app.paletteOpen) setSidebar(false);
      return;
    }
    const ctrl = e.ctrlKey && !e.metaKey;
    const mod = isMod(e);
    const handled = (fn: () => void) => {
      e.preventDefault();
      fn();
    };
    if (mod && !e.shiftKey && e.key.toLowerCase() === 'k') handled(() => (app.paletteOpen = !app.paletteOpen));
    else if (mod && !e.shiftKey && e.key.toLowerCase() === 'j') handled(toggleChat);
    else if (mod && !e.shiftKey && e.key === ',') handled(() => (app.customizeOpen = !app.customizeOpen));
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
  <ThreadDialog />
  <DrawingDialog />
  <InkDrops />
  {#if app.paletteOpen}<CommandPalette />{/if}
  <CustomizeDialog />
  <MindDialog />
  <Toast />
{/if}
