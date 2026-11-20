<script lang="ts">
  import { find } from '../state/threads.svelte';
  import Thread from './Thread.svelte';

  let { id }: { id: string } = $props();
  const thread = $derived(find(id));
</script>

{#if thread}<Thread {thread} variant="card" />{/if}
