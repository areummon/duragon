<script lang="ts" module>
  export type ThreadAction = 'full' | 'resolve' | 'copy' | 'speak' | 'note' | 'margin' | 'dismiss';
</script>

<script lang="ts">
  import type { Thread } from '../api/types';
  import { makeNote, moveToMargin, setStatus, threads, threadText } from '../state/threads.svelte';
  import { readAloud, stopSpeaking, voice } from '../state/voice.svelte';

  interface Props {
    thread: Thread;
    actions: ThreadAction[];
    onclose: () => void;
    /** Lets the thread fade out before it goes; defaults to going at once. */
    onleave?: (status: 'resolved' | 'dismissed') => void;
  }
  let { thread, actions, onclose, onleave = (s) => void setStatus(thread.id, s) }: Props = $props();

  const last = $derived([...thread.messages].reverse().find((m) => m.author === 'riddled'));
  const playing = $derived(voice.playing && !!last && voice.speakingId === last.id);

  const LABEL: Record<ThreadAction, () => string> = {
    full: () => 'Open full screen',
    resolve: () => 'Resolve',
    copy: () => 'Copy',
    speak: () => (playing ? 'Stop reading' : 'Read aloud'),
    note: () => 'Make a note',
    margin: () => 'Move to margin',
    dismiss: () => 'Dismiss',
  };
  const RUN: Record<ThreadAction, () => void> = {
    full: () => (threads.fullId = thread.id),
    resolve: () => onleave('resolved'),
    copy: () => void navigator.clipboard?.writeText(threadText(thread)),
    speak: () => {
      if (!last) return;
      if (playing) void stopSpeaking();
      else void readAloud(last.id, last.text);
    },
    note: () => void makeNote(thread.id),
    margin: () => void moveToMargin(thread.id),
    dismiss: () => onleave('dismissed'),
  };

  function run(a: ThreadAction) {
    onclose();
    RUN[a]();
  }
</script>

<div class="popover menu" role="menu">
  {#each actions as a (a)}
    <button class="menu-item" role="menuitem" onclick={() => run(a)}>{LABEL[a]()}</button>
  {/each}
</div>

<style>
  .menu {
    position: absolute;
    top: 100%;
    right: 0;
    z-index: 40;
    margin-top: 4px;
    min-width: 148px;
    white-space: nowrap;
    font-family: var(--font-sans);
    font-size: 13px;
    text-align: left;
    animation: settle var(--dur-settle) var(--ease-settle) both;
  }
  .menu-item { color: var(--graphite); }
  .menu-item:hover { color: var(--ink); }
</style>
