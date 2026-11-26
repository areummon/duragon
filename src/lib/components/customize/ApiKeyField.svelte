<script lang="ts">
  import { tick, type Snippet } from 'svelte';
  import { reasonOf } from '../../state/settings.svelte';
  import Field from './Field.svelte';

  interface Props {
    saved: boolean;
    hint?: string;
    envVar?: string | null;
    autofocus?: boolean;
    externalError?: string | null;
    onsave: (key: string) => Promise<void>;
    onremove?: () => Promise<void>;
    beforeRemove?: Snippet;
  }
  let {
    saved,
    hint,
    envVar = null,
    autofocus = false,
    externalError = null,
    onsave,
    onremove,
    beforeRemove,
  }: Props = $props();

  let draft = $state('');
  let localError = $state<string | null>(null);
  let input = $state<HTMLInputElement>();
  let busy = false;

  const shownError = $derived(localError ?? externalError);

  $effect(() => {
    if (!autofocus) return;
    void tick().then(() => input?.focus({ preventScroll: true }));
  });

  async function attempt(act: () => Promise<void>, failed: (reason: string) => string) {
    if (busy) return;
    busy = true;
    localError = null;
    try {
      await act();
    } catch (e) {
      localError = failed(reasonOf(e));
    } finally {
      busy = false;
    }
  }

  function save() {
    const trimmed = draft.trim();
    if (!trimmed) return;
    void attempt(
      async () => {
        await onsave(trimmed);
        draft = '';
        input?.focus();
      },
      (reason) => `The key wasn’t saved: ${reason}.${saved ? ' Your old key is still in place.' : ''}`,
    );
  }

  export function remove() {
    if (!onremove) return;
    void attempt(
      async () => {
        await onremove();
        input?.focus();
      },
      (reason) => `The key wasn’t removed: ${reason}.`,
    );
  }
</script>

<Field
  label="API key"
  {hint}
  type="password"
  placeholder={saved ? 'Paste a new key to replace it' : 'Paste a key'}
  bind:value={draft}
  bind:input
  error={shownError}
  onkeydown={(e) => e.key === 'Enter' && !e.isComposing && save()}
>
  {#snippet after()}<button class="solid" disabled={!draft.trim()} onclick={save}>Save</button>{/snippet}
</Field>

{#if envVar}
  <p class="help">Using the one in {envVar}. A key saved here is used instead.</p>
{/if}

{#if beforeRemove}
  <div class="foot">
    {@render beforeRemove()}
    {#if saved && onremove}
      <button class="quiet push" onclick={remove}>Remove key</button>
    {/if}
  </div>
{/if}

<style>
  .help { margin-top: 7px; font-size: 13px; line-height: 1.4; color: var(--faint); }
  .foot { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 16px; margin-top: 12px; font-size: 13px; line-height: 1.3; color: var(--graphite); }
  .push { margin-left: auto; }
  .quiet { flex: none; color: var(--graphite); transition: color 150ms var(--ease-ui); }
  @media (hover: hover) and (pointer: fine) {
    .quiet:hover { color: var(--ink); }
  }
</style>
