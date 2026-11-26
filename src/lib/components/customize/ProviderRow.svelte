<script lang="ts">
  import ChevronRight from 'lucide-svelte/icons/chevron-right';
  import { tick } from 'svelte';
  import { accessProblem, accessText, DEFAULT_PROVIDERS, hostOf, isHttpKind, isServerKind, type ProviderView } from '../../api/providers';
  import type { ProviderConfig, ProviderKind } from '../../api/types';
  import { unfold } from '../../motion/transitions';
  import { checkServers, deleteProviderKey, reasonOf, searchStatus, setProviderKey, settings, updateSettings } from '../../state/settings.svelte';
  import ApiKeyField from './ApiKeyField.svelte';
  import Field from './Field.svelte';
  import { follow } from './follow';

  interface Props {
    view: ProviderView;
    open: boolean;
    ontoggle: () => void;
    onremove: () => Promise<void>;
  }
  let { view, open, ontoggle, onremove }: Props = $props();

  const SIGN_IN: Partial<Record<ProviderKind, string>> = {
    'claude-cli': 'Uses your Claude subscription. Run claude once in a terminal to sign in.',
    'codex-cli': 'Uses your ChatGPT subscription. Run codex login once in a terminal to sign in.',
    'agy-acp': 'Uses your Antigravity account. Run agy once in a terminal to sign in.',
  };
  const SERVERS: Record<string, string> = {
    ollama: 'Get a model with ollama pull, and it shows up here.',
    'llama-cpp': 'Start llama-server with a model, and it shows up here.',
    vllm: 'Start vllm serve with a model, and it shows up here.',
  };
  const LIST = new Intl.ListFormat('en-GB');

  const p = $derived(view.provider);
  const preset = $derived(DEFAULT_PROVIDERS.find((d) => d.id === p.id));
  const saved = $derived(view.saved);
  const jobs = $derived(view.jobs.length ? `for ${LIST.format(view.jobs.map((j) => j.name))}` : '');

  let removeError = $state<string | null>(null);
  let removing = false;
  let changing = $state(false);
  let keyField = $state<ApiKeyField>();
  let addressInput = $state<HTMLInputElement>();

  function patch(change: Partial<ProviderConfig>) {
    void updateSettings({ providers: settings.value.providers.map((x) => (x.id === p.id ? { ...x, ...change } : x)) });
  }

  /** A blank goes back to what the provider came with. */
  function commit(text: string, field: 'baseUrl' | 'command') {
    const value = text.trim() || preset?.[field] || p[field] || '';
    patch({ [field]: value });
    return value;
  }

  function commitOptions(text: string) {
    const args = text.trim().split(/\s+/).filter(Boolean);
    patch({ args });
    return args.join(' ');
  }

  async function remove() {
    if (removing) return;
    removing = true;
    removeError = null;
    try {
      await onremove();
    } catch (e) {
      removeError = `The provider wasn’t removed: ${reasonOf(e)}.`;
    } finally {
      removing = false;
    }
  }

  async function change() {
    changing = true;
    await tick();
    addressInput?.focus({ preventScroll: true });
  }
</script>

<button class="head" id="provider-{p.id}" aria-expanded={open} onclick={ontoggle}>
  <span class="name" title={p.name}>{p.name}</span>
  <span class="use">{jobs}</span>
  <span class="status"><i class="dot" class:shown={accessProblem(view)}></i>{accessText(view.access)}</span>
  <span class="chev" class:open><ChevronRight size={14} strokeWidth={1.7} /></span>
</button>

{#if open}
  <div class="fold" transition:unfold={{ duration: 200 }} onintrostart={(e) => follow(e.currentTarget)}>
    <div class="body">
      {#if isHttpKind(p.kind)}
        <ApiKeyField
          bind:this={keyField}
          {saved}
          envVar={view.access.kind === 'system' ? view.access.envVar : null}
          externalError={removeError}
          onsave={(key) => setProviderKey(p.id, key)}
          onremove={() => deleteProviderKey(p.id)}
        />
        {#if changing || !p.baseUrl}
          <div class="fold" transition:unfold={{ duration: 200 }} onintrostart={(e) => follow(e.currentTarget)}>
            <div class="gap"><Field label="Address" value={p.baseUrl ?? ''} bind:input={addressInput} oncommit={(t) => commit(t, 'baseUrl')} /></div>
          </div>
        {/if}
      {:else if isServerKind(p.kind)}
        <Field label="Address" value={p.baseUrl ?? ''} error={view.access.kind === 'stopped' ? `${view.access.reason}.` : null} oncommit={(t) => commit(t, 'baseUrl')}>
          {#snippet after()}<button class="solid" onclick={() => void checkServers(p.id)}>Check</button>{/snippet}
        </Field>
        <p class="help">{SERVERS[p.id] ?? 'Its models show up here while it runs.'} {searchStatus.value?.available ? 'Uses your web search engine when Web is on.' : 'It can’t search the web.'}</p>
        {#if p.kind === 'local'}
          <div class="gap">
            <ApiKeyField
              bind:this={keyField}
              {saved}
              hint="Only if it asks for one"
              externalError={removeError}
              onsave={(key) => setProviderKey(p.id, key)}
              onremove={() => deleteProviderKey(p.id)}
            />
          </div>
        {/if}
      {:else}
        <div class="two">
          <Field label="Command" value={p.command ?? ''} oncommit={(t) => commit(t, 'command')} error={removeError} />
          {#if p.kind !== 'claude-cli'}<Field label="Options" value={p.args.join(' ')} oncommit={commitOptions} />{/if}
        </div>
        {#if SIGN_IN[p.kind]}<p class="help">{SIGN_IN[p.kind]}</p>{/if}
      {/if}
      {#if (p.baseUrl && isHttpKind(p.kind)) || saved || !p.builtin}
        <div class="foot">
          {#if p.baseUrl && isHttpKind(p.kind)}
            <span class="to">
              <span class="host" title={p.baseUrl}>Connects to {hostOf(p.baseUrl)}</span>
              {#if !changing}<span class="sep">·</span><button class="quiet" aria-label="Change the address" onclick={change}>Change</button>{/if}
            </span>
          {/if}
          {#if saved}<button class="quiet push" onclick={() => keyField?.remove()}>Remove key</button>{/if}
          {#if !p.builtin}<button class="quiet danger" class:push={!saved} onclick={remove}>Remove provider</button>{/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 42px;
    padding: 0 14px 0 16px;
    text-align: left;
    font-size: 13.5px;
    line-height: 1.3;
  }
  .head:focus-visible { outline-offset: -4px; border-radius: 11px; }
  .name, .use { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .name { flex: 0 1 auto; font-weight: 500; }
  .use { flex: 1; color: var(--faint); }
  .status { flex: none; display: inline-flex; align-items: center; gap: 7px; font-size: 13px; color: var(--graphite); white-space: nowrap; font-variant-numeric: tabular-nums; transition: color 180ms var(--ease-ui); }
  .dot { width: 6px; height: 6px; border-radius: 50%; background: transparent; transition: background-color 180ms var(--ease-ui); }
  .dot.shown { background: var(--bulb); }
  .chev { display: grid; place-items: center; flex: none; margin-left: 2px; color: var(--faint); transition: transform 160ms var(--ease-ui); }
  .chev.open { transform: rotate(90deg); }

  /* Padding on an unfolding node would stop the fold short, so it sits inside. */
  .fold { display: flow-root; scroll-margin-bottom: 12px; }
  .body { padding: 2px 16px 16px; }
  .gap { padding-top: 14px; }
  .two { display: grid; grid-auto-columns: minmax(0, 1fr); grid-auto-flow: column; gap: 10px; }
  .help { margin-top: 7px; font-size: 13px; line-height: 1.4; color: var(--faint); }
  .foot { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 16px; margin-top: 14px; font-size: 13px; line-height: 1.3; color: var(--graphite); }
  .to { display: flex; min-width: 0; max-width: 100%; gap: 6px; }
  .host { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sep { color: var(--faint); }
  .push { margin-left: auto; }
  .quiet { flex: none; color: var(--graphite); transition: color 150ms var(--ease-ui); }
  .quiet:hover { color: var(--ink); }
  .quiet.danger:hover { color: var(--text-red); }
  @media (prefers-reduced-motion: reduce) { .chev { transition: none; } }
</style>
