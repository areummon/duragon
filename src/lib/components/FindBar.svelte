<script lang="ts">
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import ChevronUp from 'lucide-svelte/icons/chevron-up';
  import Search from 'lucide-svelte/icons/search';
  import X from 'lucide-svelte/icons/x';
  import { closeOnEscape } from '../escape';
  import {
    closeFind,
    confirmFind,
    find,
    findFocus,
    nextMatch,
    prevMatch,
    setFindQuery,
  } from '../editor/find.svelte';
  import { rise } from '../motion/transitions';
  import { settings } from '../state/settings.svelte';

  let input = $state<HTMLInputElement>();

  const hasQuery = $derived(find.query.length > 0);
  const countLabel = $derived(
    find.matches.length > 0 ? `${find.index + 1} / ${find.matches.length}` : '0 / 0',
  );

  // Register before focusing the input so Escape returns focus to the editor.
  $effect(() => closeOnEscape(() => closeFind()));

  $effect(() => {
    void findFocus.seq;
    if (!input) return;
    input.focus();
    input.select();
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (e.shiftKey) {
        prevMatch();
      } else if (settings.value.vimMode) {
        confirmFind();
      } else {
        nextMatch();
      }
      return;
    }
    if (e.key === 'Escape') {
      // Prevent browser's built-in search-input clear; let Escape bubble to closeTop().
      e.preventDefault();
    }
  }
</script>

<div
  class="find-bar"
  role="search"
  aria-label="Find in this note"
  data-testid="find-bar"
  transition:rise={{ y: -4, duration: 150 }}
>
  <span class="icon" aria-hidden="true">
    <Search size={14} strokeWidth={1.6} />
  </span>
  <input
    bind:this={input}
    type="search"
    value={find.query}
    oninput={(e) => setFindQuery(e.currentTarget.value)}
    {onkeydown}
    aria-label="Find in this note"
    placeholder="Find in note…"
    spellcheck="false"
    autocomplete="off"
    data-testid="find-input"
  />
  {#if hasQuery}
    <span class="count" class:empty={find.matches.length === 0} aria-live="polite" data-testid="find-count">
      {countLabel}
    </span>
  {/if}
  <div class="actions">
    <button
      type="button"
      class="nav"
      aria-label="Previous match"
      title="Previous match (Shift+Enter)"
      disabled={find.matches.length === 0}
      onclick={() => prevMatch()}
      data-testid="find-prev"
    >
      <ChevronUp size={14} strokeWidth={1.6} />
    </button>
    <button
      type="button"
      class="nav"
      aria-label="Next match"
      title="Next match (Enter)"
      disabled={find.matches.length === 0}
      onclick={() => nextMatch()}
      data-testid="find-next"
    >
      <ChevronDown size={14} strokeWidth={1.6} />
    </button>
    <button
      type="button"
      class="nav"
      aria-label="Close find"
      title="Close (Esc)"
      onclick={() => closeFind({ focusEditor: true })}
      data-testid="find-close"
    >
      <X size={14} strokeWidth={1.6} />
    </button>
  </div>
</div>

<style>
  .find-bar {
    position: fixed;
    top: 64px;
    right: 24px;
    z-index: 35;
    display: flex;
    align-items: center;
    gap: 6px;
    height: 38px;
    padding: 0 5px 0 11px;
    border: 1px solid var(--rule);
    border-radius: 11px;
    background: var(--paper);
    box-shadow: var(--lift);
  }
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--faint);
    flex: none;
  }
  input {
    width: 184px;
    min-width: 120px;
    height: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    font: inherit;
    font-size: 13.5px;
    line-height: 1.3;
    color: var(--ink);
    outline: none;
  }
  input::placeholder {
    color: var(--faint);
  }
  input::-webkit-search-cancel-button,
  input::-webkit-search-decoration {
    -webkit-appearance: none;
    appearance: none;
  }
  .count {
    flex: none;
    padding: 0 4px;
    font-size: 12px;
    line-height: 1.3;
    font-variant-numeric: tabular-nums;
    color: var(--graphite);
    user-select: none;
  }
  .count.empty {
    color: var(--faint);
  }
  .actions {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    flex: none;
  }
  .nav {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--graphite);
    flex: none;
    transition:
      color var(--dur-ui) var(--ease-ui),
      background-color var(--dur-ui) var(--ease-ui),
      transform 140ms var(--ease-step);
  }
  @media (hover: hover) and (pointer: fine) {
    .nav:hover:not(:disabled) {
      color: var(--ink);
      background: var(--riddle-wash);
    }
  }
  .nav:active:not(:disabled) {
    transform: scale(0.94);
  }
  .nav:disabled {
    opacity: 0.35;
    cursor: default;
  }
  @media (prefers-reduced-motion: reduce) {
    .nav:active:not(:disabled) {
      transform: none;
    }
  }
  @media print {
    .find-bar {
      display: none;
    }
  }
</style>
