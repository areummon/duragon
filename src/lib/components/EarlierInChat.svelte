<script lang="ts">
  import ChevronRight from 'lucide-svelte/icons/chevron-right';
  import { chat, saveSummary } from '../state/chat.svelte';

  let open = $state(false);
  let focused = $state(false);
  let draft = $state(chat.summary);
  let timer = 0;

  $effect(() => {
    const s = chat.summary;
    if (!focused) draft = s;
  });

  function input(e: Event) {
    draft = (e.currentTarget as HTMLTextAreaElement).value;
    const noteId = chat.noteId;
    window.clearTimeout(timer);
    if (noteId) timer = window.setTimeout(() => void saveSummary(noteId, draft), 300);
  }
  const commit = () => {
    focused = false;
    window.clearTimeout(timer);
    if (chat.noteId && draft !== chat.summary) void saveSummary(chat.noteId, draft);
  };
</script>

{#if chat.summary || focused}
  <div class="earlier">
    <button class="row" aria-expanded={open} onclick={() => (open = !open)}>
      <span class="chev" class:open><ChevronRight size={14} strokeWidth={1.6} /></span>Earlier in this chat
    </button>
    {#if open}
      <p class="note">What I remember of the start of this chat. Edit it to change what I keep.</p>
      <textarea
        value={draft}
        oninput={input}
        onfocus={() => (focused = true)}
        onblur={commit}
        rows="6"
        aria-label="Earlier in this chat"
      ></textarea>
    {/if}
  </div>
{/if}

<style>
  .earlier { display: flex; flex-direction: column; gap: 8px; margin-bottom: -6px; }
  .row { display: flex; align-items: center; gap: 6px; align-self: flex-start; font-size: 13px; line-height: 1.3; color: var(--faint); transition: color var(--dur-ui) var(--ease-ui); }
  .row:hover, .row[aria-expanded='true'] { color: var(--ink); }
  .chev { display: grid; place-items: center; transition: transform 160ms var(--ease-ui); }
  .chev.open { transform: rotate(90deg); }
  .note { font-size: 12.5px; line-height: 1.45; color: var(--faint); }
  textarea {
    display: block;
    width: 100%;
    padding: 12px 14px;
    border: 1px solid var(--rule);
    border-radius: 12px;
    background: transparent;
    font: inherit;
    font-size: 14px;
    line-height: 1.55;
    color: var(--ink);
    outline: none;
    resize: vertical;
    transition: border-color 150ms var(--ease-ui);
  }
  textarea:focus { border-color: var(--faint); }

  @media (prefers-reduced-motion: reduce) {
    .chev { transition: none; }
  }
</style>
