<script lang="ts">
  import { closeOnEscape } from '../escape';
  import { takeFocus } from '../focus';
  import { pop, veil } from '../motion/transitions';
  import { find, threads } from '../state/threads.svelte';
  import CloseButton from './CloseButton.svelte';
  import Thread from './Thread.svelte';

  const thread = $derived(find(threads.fullId));
  const close = () => (threads.fullId = null);
  $effect(() => {
    if (threads.fullId) return closeOnEscape(close);
  });
</script>

{#if thread}
  <div class="scrim" role="presentation" onclick={(e) => e.target === e.currentTarget && close()} transition:veil|global>
    <div class="card" role="dialog" aria-modal="true" aria-label="Thread" tabindex="-1" use:takeFocus in:pop|global out:pop={{ duration: 140 }}>
      <CloseButton class="close" onclick={close} />
      <blockquote>{thread.quote}</blockquote>
      <Thread {thread} variant="dialog" />
    </div>
  </div>
{/if}

<style>
  .scrim { position: fixed; inset: 0; z-index: 55; display: grid; place-items: center; padding: 24px; background: color-mix(in srgb, var(--paper) 55%, transparent); }
  .card { position: relative; width: min(720px, 100%); max-height: calc(100vh - 48px); overflow: auto; padding: 30px 34px 28px; border-radius: 14px; background: var(--paper); box-shadow: var(--lift); outline: none; }
  .card > :global(.close) {
    position: absolute;
    top: 14px;
    right: 14px;
  }
  blockquote { margin: 0 0 20px; padding-left: 14px; border-left: 2px solid var(--rule); font-size: 14px; line-height: 1.5; color: var(--faint); }
</style>
