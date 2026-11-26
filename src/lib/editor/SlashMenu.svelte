<script lang="ts">
  import type { Editor } from '@tiptap/core';
  import InlineMenuPopup from './InlineMenuPopup.svelte';
  import { slashMenu } from './slash.svelte';

  let { editor }: { editor: Editor } = $props();
</script>

<InlineMenuPopup {editor} menu={slashMenu} label="Commands" itemKey={(item) => item.id}>
  {#snippet children(item)}
    {@const Icon = item.icon}
    <Icon size={15} strokeWidth={1.5} />
    <span>{item.label}</span>
  {/snippet}
</InlineMenuPopup>
