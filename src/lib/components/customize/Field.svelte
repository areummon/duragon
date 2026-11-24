<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends Omit<HTMLInputAttributes, 'onchange'> {
    label: string;
    hint?: string;
    value?: string;
    error?: string | null;
    input?: HTMLInputElement;
    /** Takes the edited text, and gives back what it was stored as. */
    oncommit?: (text: string) => string;
    after?: Snippet;
  }
  let { label, hint, value = $bindable(''), error, input = $bindable(), oncommit, after, ...rest }: Props = $props();
  const id = $props.id();
</script>

<div>
  <label for={id}>{label}{#if hint}<span class="aside">{hint}</span>{/if}</label>
  <div class="line">
    <input
      {...rest}
      {id}
      class:bad={error}
      bind:this={input}
      bind:value
      onchange={() => { if (oncommit) value = oncommit(value); }}
      aria-invalid={error ? true : undefined}
      aria-describedby={error ? `${id}-error` : undefined}
      spellcheck="false"
      autocomplete="off"
    />
    {@render after?.()}
  </div>
  {#if error}<p id="{id}-error" class="error" role="alert">{error}</p>{/if}
</div>

<style>
  label { display: block; margin-bottom: 6px; font-size: 13px; line-height: 1.3; color: var(--graphite); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .aside { margin-left: 4px; color: var(--faint); }
  .line { display: flex; align-items: center; gap: 8px; }
  input {
    flex: 1;
    min-width: 0;
    height: 34px;
    padding: 0 11px;
    border: 1px solid var(--rule);
    border-radius: 9px;
    background: transparent;
    font: inherit;
    font-size: 13px;
    line-height: 1.3;
    color: var(--ink);
    outline: none;
    transition: border-color 150ms var(--ease-ui);
  }
  input::placeholder { color: var(--faint); opacity: 1; }
  input:focus { border-color: var(--faint); }
  input.bad { border-color: var(--text-red); }
  .error { margin-top: 7px; font-size: 13px; line-height: 1.4; color: var(--text-red); overflow-wrap: anywhere; }
</style>
