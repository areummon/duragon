<script lang="ts">
  import { isHttpKind } from '../../api/providers';
  import type { ProviderConfig, ProviderKind } from '../../api/types';
  import Field from './Field.svelte';
  import Pills from './Pills.svelte';

  let { onadd, oncancel }: { onadd: (provider: ProviderConfig) => void; oncancel: () => void } = $props();

  const KINDS: { id: ProviderKind; name: string }[] = [
    { id: 'openai', name: 'OpenAI' },
    { id: 'anthropic', name: 'Anthropic' },
    { id: 'gemini', name: 'Gemini' },
    { id: 'acp', name: 'A command on this computer' },
  ];

  let kind = $state<ProviderKind>('openai');
  let name = $state('');
  let address = $state('');
  let command = $state('');
  let options = $state('');
  let models = $state('');
  let nameInput = $state<HTMLInputElement>();

  const http = $derived(isHttpKind(kind));
  const ready = $derived(!!name.trim() && !!(http ? address : command).trim());

  $effect(() => nameInput?.focus({ preventScroll: true }));

  const words = (text: string, by: RegExp) => text.split(by).map((w) => w.trim()).filter(Boolean);

  function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!ready) return;
    // Never an id a removed provider had, whose key could still be around.
    const id = `${name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'custom'}-${Date.now().toString(36)}`;
    onadd({
      id,
      name: name.trim(),
      kind,
      baseUrl: http ? address.trim() : null,
      command: http ? null : command.trim(),
      args: http ? [] : words(options, /\s+/),
      envVar: null,
      models: [...new Set(words(models, /,/))],
      builtin: false,
    });
  }
</script>

<form onsubmit={submit} aria-label="New provider">
  <h5>New provider</h5>
  <div>
    <div class="label" id="works-like">Works like</div>
    <Pills labelledby="works-like" options={KINDS} value={kind} onpick={(k) => (kind = k)} />
  </div>
  <div class="two">
    <Field label="Name" placeholder="Groq" bind:value={name} bind:input={nameInput} />
    {#if http}
      <Field label="Address" placeholder={kind === 'openai' ? 'https://api.groq.com/openai/v1' : 'https://api.example.com'} bind:value={address} />
    {:else}
      <Field label="Command" placeholder="my-agent" bind:value={command} />
    {/if}
  </div>
  <div class:two={!http}>
    {#if !http}<Field label="Options" placeholder="acp" bind:value={options} />{/if}
    <Field label="Models" hint="Optional, separated by commas" placeholder="llama-3.3-70b-versatile, openai/gpt-oss-120b" bind:value={models} />
  </div>
  <div class="actions">
    <button type="button" class="quiet" onclick={oncancel}>Cancel</button>
    <button class="solid" disabled={!ready}>Add provider</button>
  </div>
</form>

<style>
  form { display: flex; flex-direction: column; gap: 14px; padding: 0 16px 16px; }
  h5 { display: flex; align-items: center; min-height: 38px; margin: 0 0 -12px; font-size: 13.5px; line-height: 1.3; font-weight: 500; }
  .label { margin-bottom: 6px; font-size: 13px; line-height: 1.3; color: var(--graphite); }
  .two { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 10px; }
  .actions { display: flex; align-items: center; justify-content: flex-end; gap: 8px; }
  .quiet { height: 34px; padding: 0 8px; font-size: 13px; color: var(--graphite); transition: color 150ms var(--ease-ui); }
  .quiet:hover { color: var(--ink); }
</style>
