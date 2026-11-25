<script lang="ts">
  import type { Component } from 'svelte';
  import { closeOnEscape } from '../escape';

  interface Props {
    open: boolean;
    close: () => void;
    load: () => Promise<{ default: Component }>;
    remount?: boolean;
  }
  let { open, close, load, remount = false }: Props = $props();

  // The first opening mounts the dialog already open, so its entrance is a global transition.
  let Dialog = $state.raw<Component>();
  const loading = $derived(open && !Dialog);

  $effect(() => {
    if (!loading) return;
    void load().then((m) => (Dialog = m.default), close);
    return closeOnEscape(close);
  });

  function swallow(e: KeyboardEvent) {
    if (!loading || e.key === 'Escape') return;
    e.preventDefault();
    e.stopImmediatePropagation();
  }
</script>

<svelte:window onkeydowncapture={swallow} />

{#if loading}
  <div class="loading" role="dialog" aria-modal="true" aria-busy="true" aria-label="Opening"></div>
{:else if Dialog && (open || !remount)}
  <Dialog />
{/if}

<style>
  .loading { position: fixed; inset: 0; z-index: 75; }
</style>
