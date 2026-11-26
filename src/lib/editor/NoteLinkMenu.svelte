<script lang="ts">
  import type { Editor } from '@tiptap/core';
  import FileText from 'lucide-svelte/icons/file-text';
  import InlineMenuPopup from './InlineMenuPopup.svelte';
  import { noteLinkMenu } from './notelink.svelte';

  let { editor }: { editor: Editor } = $props();
</script>

<InlineMenuPopup {editor} menu={noteLinkMenu} label="Link to note" itemKey={(item) => item.id} animate>
  {#snippet children(item)}
    <FileText size={15} strokeWidth={1.5} />
    <span class="label">{item.title || 'Untitled'}</span>
  {/snippet}
</InlineMenuPopup>

<style>
  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
