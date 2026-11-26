<script lang="ts">
  import Check from 'lucide-svelte/icons/check';
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import ChevronRight from 'lucide-svelte/icons/chevron-right';
  import type { SearchEngine, SearchEngineCredentialStatus } from '../../api/types';
  import { unfold } from '../../motion/transitions';
  import { deleteSearchKey, reasonOf, searchStatus, setSearchKey, settings, updateSettings } from '../../state/settings.svelte';
  import Popover from '../Popover.svelte';
  import Field from './Field.svelte';
  import { follow } from './follow';
  import Reach from './Reach.svelte';

  const FALLBACK_ENGINES: SearchEngineCredentialStatus[] = [
    { engine: 'tavily', label: 'Tavily', envVar: 'TAVILY_API_KEY', docsUrl: 'https://app.tavily.com/', hasStoredKey: false, maskedKey: null, hasEnvKey: false },
    { engine: 'brave', label: 'Brave Search', envVar: 'BRAVE_API_KEY', docsUrl: 'https://brave.com/search/api/', hasStoredKey: false, maskedKey: null, hasEnvKey: false },
    { engine: 'exa', label: 'Exa', envVar: 'EXA_API_KEY', docsUrl: 'https://dashboard.exa.ai/api-keys', hasStoredKey: false, maskedKey: null, hasEnvKey: false },
    { engine: 'firecrawl', label: 'Firecrawl', envVar: 'FIRECRAWL_API_KEY', docsUrl: 'https://www.firecrawl.dev/app/api-keys', hasStoredKey: false, maskedKey: null, hasEnvKey: false },
    { engine: 'ollama', label: 'Ollama', envVar: 'OLLAMA_API_KEY', docsUrl: 'https://ollama.com/settings/keys', hasStoredKey: false, maskedKey: null, hasEnvKey: false },
  ];

  const status = $derived(searchStatus.value);
  const engines = $derived(status?.engines ?? FALLBACK_ENGINES);
  const pinned = $derived(settings.value.webSearchEngine);
  const pinnedLabel = $derived(pinned ? (engines.find((e) => e.engine === pinned)?.label ?? pinned) : 'Automatic');
  const statusLine = $derived(status?.reason ?? 'No search engine key set');
  const missingPinned = $derived(!!(status?.pinned && !status.available));

  let menuOpen = $state(false);
  let openRows = $state<Partial<Record<SearchEngine, boolean>>>({});
  let drafts = $state<Partial<Record<SearchEngine, string>>>({});
  let errors = $state<Partial<Record<SearchEngine, string | null>>>({});
  let inputs = $state<Partial<Record<SearchEngine, HTMLInputElement>>>({});
  const busy: Partial<Record<SearchEngine, boolean>> = {};

  const hasKey = (e: SearchEngineCredentialStatus) => e.hasStoredKey || e.hasEnvKey;

  function keyStatusText(e: SearchEngineCredentialStatus): string {
    if (e.hasStoredKey) return e.maskedKey ?? 'Key saved';
    if (e.hasEnvKey) return 'Key from your system';
    if (e.error) return 'Key can’t be read';
    return 'Not set';
  }

  function pickEngine(engine: SearchEngine | null) {
    menuOpen = false;
    void updateSettings({ webSearchEngine: engine });
  }

  async function attempt(engine: SearchEngine, act: () => Promise<void>, failed: (reason: string) => string) {
    if (busy[engine]) return;
    busy[engine] = true;
    errors[engine] = null;
    try {
      await act();
    } catch (err) {
      errors[engine] = failed(reasonOf(err));
    } finally {
      busy[engine] = false;
    }
  }

  function saveKey(e: SearchEngineCredentialStatus) {
    const trimmed = (drafts[e.engine] ?? '').trim();
    if (!trimmed) return;
    void attempt(
      e.engine,
      async () => {
        await setSearchKey(e.engine, trimmed);
        drafts[e.engine] = '';
        inputs[e.engine]?.focus();
      },
      (reason) => `The key wasn’t saved: ${reason}.${e.hasStoredKey ? ' Your old key is still in place.' : ''}`,
    );
  }

  function removeKey(e: SearchEngineCredentialStatus) {
    void attempt(
      e.engine,
      async () => {
        await deleteSearchKey(e.engine);
        inputs[e.engine]?.focus();
      },
      (reason) => `The key wasn’t removed: ${reason}.`,
    );
  }
</script>

<div class="group">
  <div class="top">
    <div>
      <h4>Web search engine</h4>
      <p class="note">
        {#if missingPinned}<i class="dot shown"></i>{/if}{statusLine}
      </p>
    </div>
    <div class="picker">
      <button
        class="ghost"
        class:open={menuOpen}
        aria-label="Search engine"
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        onclick={() => (menuOpen = !menuOpen)}
      >
        <span>{pinnedLabel}</span>
        <ChevronDown size={14} strokeWidth={1.7} />
      </button>
      <Popover open={menuOpen} onclose={() => (menuOpen = false)} align="right" width={230} label="Search engine">
        <button class="menu-item" role="menuitem" onclick={() => pickEngine(null)}>
          <span class="choice-label">Automatic</span>
          {#if pinned === null}<span class="check"><Check size={14} strokeWidth={1.8} /></span>{/if}
        </button>
        <div class="menu-sep"></div>
        {#each engines as e (e.engine)}
          <button class="menu-item" role="menuitem" onclick={() => pickEngine(e.engine)}>
            <span class="choice-label">{e.label}</span>
            {#if hasKey(e)}<span class="choice-meta">Key set</span>{/if}
            {#if pinned === e.engine}<span class="check"><Check size={14} strokeWidth={1.8} /></span>{/if}
          </button>
        {/each}
      </Popover>
      {#if menuOpen}<Reach />{/if}
    </div>
  </div>

  {#if searchStatus.error}
    <p class="error" role="alert">{searchStatus.error}.</p>
  {/if}

  <div class="list">
    {#each engines as e (e.engine)}
      {@const open = !!openRows[e.engine]}
      {@const active = status?.engine === e.engine}
      {@const missing = status?.pinned === e.engine && !hasKey(e)}
      <div class="item">
        <div class="inner">
          <button
            class="head"
            id="search-engine-{e.engine}"
            aria-expanded={open}
            onclick={() => (openRows[e.engine] = !openRows[e.engine])}
          >
            <span class="name" title={e.label}>{e.label}</span>
            <span class="use">{active ? 'Active' : ''}</span>
            <span class="status"><i class="dot" class:shown={missing}></i>{keyStatusText(e)}</span>
            <span class="chev" class:open><ChevronRight size={14} strokeWidth={1.7} /></span>
          </button>

          {#if open}
            <div class="fold" transition:unfold={{ duration: 200 }} onintrostart={(ev) => follow(ev.currentTarget)}>
              <div class="body">
                <Field
                  label="API key"
                  type="password"
                  placeholder={e.hasStoredKey ? 'Paste a new key to replace it' : 'Paste a key'}
                  bind:value={drafts[e.engine]}
                  bind:input={inputs[e.engine]}
                  error={errors[e.engine] ?? null}
                  onkeydown={(ev) => ev.key === 'Enter' && !ev.isComposing && saveKey(e)}
                >
                  {#snippet after()}
                    <button class="solid" disabled={!(drafts[e.engine] ?? '').trim()} onclick={() => saveKey(e)}>Save</button>
                  {/snippet}
                </Field>
                {#if e.hasEnvKey && !e.hasStoredKey}
                  <p class="help">Using the one in {e.envVar}. A key saved here is used instead.</p>
                {/if}
                {#if e.hasStoredKey}
                  <div class="foot">
                    <button class="quiet push" onclick={() => removeKey(e)}>Remove key</button>
                  </div>
                {/if}
              </div>
            </div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .group { margin-top: 22px; }
  .top { display: flex; align-items: flex-end; justify-content: space-between; gap: 20px; }
  h4 { margin: 0; font-size: 13.5px; line-height: 1.3; font-weight: 500; color: var(--ink); }
  .note { display: flex; align-items: center; gap: 7px; margin-top: 3px; font-size: 13px; line-height: 1.4; color: var(--faint); }
  .picker { position: relative; flex: none; }
  .ghost {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 11px 0 13px;
    border: 1px solid var(--rule);
    border-radius: 999px;
    font-size: 13px;
    line-height: 1;
    color: var(--ink);
    transition: border-color 150ms var(--ease-ui), transform 160ms var(--ease-step);
  }
  .ghost.open { border-color: var(--faint); }
  @media (hover: hover) and (pointer: fine) { .ghost:hover { border-color: var(--faint); } }
  .ghost:active { transform: scale(0.97); }
  .picker .menu-item { display: flex; align-items: center; gap: 8px; font-size: 13.5px; line-height: 1.3; }
  .choice-label { flex: 1; min-width: 0; text-align: left; }
  .choice-meta { flex: none; font-size: 12px; color: var(--faint); }
  .check { display: grid; place-items: center; flex: none; color: var(--ink); }
  .error { margin-top: 12px; font-size: 13px; line-height: 1.4; color: var(--text-red); overflow-wrap: anywhere; }

  .list { margin-top: 12px; border: 1px solid var(--rule); border-radius: 12px; }
  .item { display: flow-root; scroll-margin-bottom: 12px; }
  .item:not(:first-child) > .inner { border-top: 1px solid var(--rule); }

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
  .dot { flex: none; width: 6px; height: 6px; border-radius: 50%; background: transparent; transition: background-color 180ms var(--ease-ui); }
  .dot.shown { background: var(--bulb); }
  .chev { display: grid; place-items: center; flex: none; margin-left: 2px; color: var(--faint); transition: transform 160ms var(--ease-ui); }
  .chev.open { transform: rotate(90deg); }

  .fold { display: flow-root; scroll-margin-bottom: 12px; }
  .body { padding: 2px 16px 16px; }
  .help { margin-top: 7px; font-size: 13px; line-height: 1.4; color: var(--faint); }
  .foot { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 16px; margin-top: 14px; font-size: 13px; line-height: 1.3; color: var(--graphite); }
  .push { margin-left: auto; }
  .quiet { flex: none; color: var(--graphite); transition: color 150ms var(--ease-ui); }
  .quiet:hover { color: var(--ink); }
  @media (prefers-reduced-motion: reduce) {
    .ghost:active { transform: none; }
    .chev { transition: none; }
  }
</style>
