<script lang="ts">
  import Check from 'lucide-svelte/icons/check';
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import ChevronRight from 'lucide-svelte/icons/chevron-right';
  import ExternalLink from 'lucide-svelte/icons/external-link';
  import { isTauri, openExternalUrl } from '../../api/platform';
  import { FALLBACK_SEARCH_ENGINES } from '../../api/search';
  import type { SearchEngine, SearchEngineCredentialStatus } from '../../api/types';
  import { unfold } from '../../motion/transitions';
  import { deleteSearchKey, searchStatus, setSearchKey, settings, updateSettings } from '../../state/settings.svelte';
  import Popover from '../Popover.svelte';
  import ApiKeyField from './ApiKeyField.svelte';
  import { follow } from './follow';
  import Reach from './Reach.svelte';

  const status = $derived(searchStatus.value);
  const engines = $derived(status?.engines ?? FALLBACK_SEARCH_ENGINES);
  const pinned = $derived(settings.value.webSearchEngine);
  const pinnedLabel = $derived(pinned ? (engines.find((e) => e.engine === pinned)?.label ?? pinned) : 'Automatic');
  const statusLine = $derived(status?.reason ?? 'No search engine key set');
  const missingPinned = $derived(!!(status?.pinned && !status.available));

  let menuOpen = $state(false);
  let openRows = $state<Partial<Record<SearchEngine, boolean>>>({});

  const hasKey = (e: SearchEngineCredentialStatus) => e.hasStoredKey || e.hasEnvKey;

  function keyStatusText(e: SearchEngineCredentialStatus): string {
    if (e.hasStoredKey) return e.maskedKey ?? 'Key saved';
    if (e.hasEnvKey) return 'Key from your system';
    if (e.error) return 'Key can’t be read';
    return 'Not set';
  }

  function toggleRow(engine: SearchEngine) {
    openRows[engine] = !openRows[engine];
  }

  function pickEngine(engine: SearchEngine | null) {
    menuOpen = false;
    void updateSettings({ webSearchEngine: engine });
    if (engine) {
      const target = engines.find((item) => item.engine === engine);
      if (target && !hasKey(target)) openRows[engine] = true;
    }
  }

  function openDocs(ev: MouseEvent, url: string) {
    if (!isTauri()) return;
    ev.preventDefault();
    void openExternalUrl(url);
  }
</script>

<div class="group">
  <div class="top">
    <div>
      <h4>Search engine</h4>
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
        <span class="trigger-chev" class:open={menuOpen}><ChevronDown size={14} strokeWidth={1.7} /></span>
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
            onclick={() => toggleRow(e.engine)}
          >
            <span class="name" title={e.label}>{e.label}</span>
            <span class="use">{#if active}<span class="pill">Active</span>{/if}</span>
            <span class="status" class:unset={!hasKey(e) && !e.error}><i class="dot" class:shown={missing}></i>{keyStatusText(e)}</span>
            <span class="chev" class:open><ChevronRight size={14} strokeWidth={1.7} /></span>
          </button>

          {#if open}
            <div class="fold" transition:unfold={{ duration: 200 }} onintrostart={(ev) => follow(ev.currentTarget)}>
              <div class="body">
                <ApiKeyField
                  saved={e.hasStoredKey}
                  envVar={e.hasEnvKey && !e.hasStoredKey ? e.envVar : null}
                  autofocus
                  onsave={(key) => setSearchKey(e.engine, key)}
                  onremove={() => deleteSearchKey(e.engine)}
                >
                  {#snippet beforeRemove()}
                    <a
                      class="docs"
                      href={e.docsUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      onclick={(ev) => openDocs(ev, e.docsUrl)}
                    >
                      <span>Get a key</span>
                      <ExternalLink size={12} strokeWidth={1.7} />
                    </a>
                  {/snippet}
                </ApiKeyField>
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
    padding: 0 10px 0 13px;
    border: 1px solid var(--rule);
    border-radius: 999px;
    font-size: 13px;
    line-height: 1;
    color: var(--ink);
    transition: border-color 150ms var(--ease-ui), background-color 150ms var(--ease-ui), transform 160ms var(--ease-step);
  }
  .ghost.open { border-color: var(--faint); background: var(--shelf); }
  @media (hover: hover) and (pointer: fine) { .ghost:hover { border-color: var(--faint); } }
  .ghost:active { transform: scale(0.97); }
  .trigger-chev { display: grid; place-items: center; color: var(--faint); transition: transform 160ms var(--ease-ui); }
  .trigger-chev.open { transform: rotate(180deg); }
  .picker .menu-item { display: flex; align-items: center; gap: 8px; font-size: 13.5px; line-height: 1.3; }
  .choice-label { flex: 1; min-width: 0; text-align: left; }
  .choice-meta { flex: none; font-size: 12px; color: var(--faint); }
  .check { display: grid; place-items: center; flex: none; color: var(--ink); }
  .error { margin-top: 12px; font-size: 13px; line-height: 1.4; color: var(--text-red); overflow-wrap: anywhere; }

  .list { margin-top: 12px; border: 1px solid var(--rule); border-radius: 12px; overflow: hidden; }
  .item { display: flow-root; scroll-margin-bottom: 12px; }
  .item:not(:first-child) > .inner { border-top: 1px solid var(--rule); }

  .head {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 44px;
    padding: 0 14px 0 16px;
    text-align: left;
    font-size: 13.5px;
    line-height: 1.3;
    transition: background-color 140ms var(--ease-ui);
  }
  @media (hover: hover) and (pointer: fine) { .head:hover { background: var(--shelf); } }
  .head:focus-visible { outline-offset: -4px; border-radius: 11px; }
  .name, .use { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .name { flex: 0 1 auto; font-weight: 500; }
  .use { flex: 1; display: flex; align-items: center; }
  .pill {
    display: inline-flex;
    align-items: center;
    padding: 3px 8px;
    border-radius: 999px;
    background: var(--shelf);
    font-size: 11.5px;
    font-weight: 500;
    line-height: 1.2;
    color: var(--ink);
    text-box: trim-both cap alphabetic;
  }
  .status { flex: none; display: inline-flex; align-items: center; gap: 7px; font-size: 13px; color: var(--graphite); white-space: nowrap; font-variant-numeric: tabular-nums; transition: color 180ms var(--ease-ui); }
  .status.unset { color: var(--faint); }
  .dot { flex: none; width: 6px; height: 6px; border-radius: 50%; background: transparent; transition: background-color 180ms var(--ease-ui); }
  .dot.shown { background: var(--bulb); }
  .chev { display: grid; place-items: center; flex: none; margin-left: 2px; color: var(--faint); transition: transform 160ms var(--ease-ui); }
  .chev.open { transform: rotate(90deg); }

  .fold { display: flow-root; scroll-margin-bottom: 12px; }
  .body { padding: 4px 16px 16px; }
  .docs { display: inline-flex; align-items: center; gap: 5px; color: var(--faint); text-decoration: none; transition: color 150ms var(--ease-ui); }
  @media (hover: hover) and (pointer: fine) {
    .docs:hover { color: var(--ink); }
  }
  @media (prefers-reduced-motion: reduce) {
    .ghost:active { transform: none; }
    .chev, .trigger-chev { transition: none; }
  }
</style>
