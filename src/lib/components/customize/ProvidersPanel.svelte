<script lang="ts">
  import Plus from 'lucide-svelte/icons/plus';
  import { tick } from 'svelte';
  import { isServerKind, type ProviderView } from '../../api/providers';
  import type { ProviderConfig } from '../../api/types';
  import { closeOnEscape } from '../../escape';
  import { unfold } from '../../motion/transitions';
  import { credentialStatuses, removeProvider, settings, updateSettings } from '../../state/settings.svelte';
  import Popover from '../Popover.svelte';
  import { follow } from './follow';
  import NewProvider from './NewProvider.svelte';
  import ProviderRow from './ProviderRow.svelte';
  import Reach from './Reach.svelte';

  let { views }: { views: ProviderView[] } = $props();

  /** The rows the writer has opened, true while open. One chosen under Add stays listed once it is closed. */
  let rows = $state<Record<string, boolean>>({});
  let menuOpen = $state(false);
  let creating = $state(false);
  let addButton = $state<HTMLElement>();

  const shown = $derived(views.filter((v) => v.listed || v.provider.id in rows));
  const others = $derived(views.filter((v) => !shown.includes(v)));
  const groups = $derived([
    { label: 'On this computer', views: others.filter((v) => isServerKind(v.provider.kind)) },
    { label: 'With an API key', views: others.filter((v) => !isServerKind(v.provider.kind)) },
  ].filter((g) => g.views.length));

  export function openAdd() {
    addButton?.focus({ preventScroll: true });
    menuOpen = true;
  }

  const head = (id: string) => document.getElementById(`provider-${id}`);

  async function openRow(id: string) {
    rows[id] = true;
    await tick();
    (head(id)?.parentElement?.querySelector('input') ?? head(id))?.focus({ preventScroll: true });
  }

  function chooseOther(id: string) {
    menuOpen = false;
    void openRow(id);
  }

  function create() {
    menuOpen = false;
    creating = true;
  }

  function cancel() {
    creating = false;
    addButton?.focus({ preventScroll: true });
  }

  async function added(provider: ProviderConfig) {
    creating = false;
    const saved = updateSettings({ providers: [...settings.value.providers, provider] });
    void openRow(provider.id);
    if (await saved) return;
    delete rows[provider.id];
    addButton?.focus({ preventScroll: true });
  }

  async function remove(id: string) {
    const i = shown.findIndex((v) => v.provider.id === id);
    const next = shown[i + 1] ?? shown[i - 1];
    await removeProvider(id);
    delete rows[id];
    head(next.provider.id)?.focus({ preventScroll: true });
  }

  $effect(() => {
    if (creating) return closeOnEscape(cancel, () => addButton);
  });
</script>

<div class="group">
  <div class="top">
    <div>
      <h4>Providers</h4>
      <p class="note">Where models come from. A key goes only to its own provider.</p>
    </div>
    <div class="adder">
      <button class="ghost" class:open={menuOpen} bind:this={addButton} aria-haspopup="menu" aria-expanded={menuOpen} onclick={() => (menuOpen = !menuOpen)}>
        <Plus size={14} strokeWidth={1.5} />Add
      </button>
      <Popover open={menuOpen} onclose={() => (menuOpen = false)} align="right" width={270} label="Add a provider">
        {#each groups as g (g.label)}
          <div class="glabel">{g.label}</div>
          {#each g.views as v (v.provider.id)}
            <button class="menu-item" role="menuitem" onclick={() => chooseOther(v.provider.id)}>{v.provider.name}</button>
          {/each}
          <div class="menu-sep"></div>
        {/each}
        <button class="menu-item" role="menuitem" onclick={create}>Something else…</button>
      </Popover>
      {#if menuOpen}<Reach />{/if}
    </div>
  </div>

  {#if credentialStatuses.error}
    <p class="error" role="alert">{credentialStatuses.error}.</p>
  {/if}

  <div class="list">
    {#if creating}
      <div class="item" transition:unfold={{ duration: 200 }} onintrostart={(e) => follow(e.currentTarget)}>
        <div class="inner"><NewProvider onadd={added} oncancel={cancel} /></div>
      </div>
    {/if}
    {#each shown as v (v.provider.id)}
      <div class="item" transition:unfold={{ duration: 200 }} onintrostart={(e) => v.provider.id in rows && follow(e.currentTarget)}>
        <div class="inner">
          <ProviderRow view={v} open={!!rows[v.provider.id]} ontoggle={() => (rows[v.provider.id] = !rows[v.provider.id])} onremove={() => remove(v.provider.id)} />
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .group { margin-top: 28px; }
  .top { display: flex; align-items: flex-end; justify-content: space-between; gap: 20px; }
  h4 { margin: 0; font-size: 13.5px; line-height: 1.3; font-weight: 500; color: var(--ink); }
  .note { margin-top: 3px; font-size: 13px; line-height: 1.4; color: var(--faint); }
  .adder { position: relative; flex: none; }
  .ghost { display: inline-flex; align-items: center; gap: 6px; height: 30px; padding: 0 14px 0 11px; border: 1px solid var(--rule); border-radius: 999px; font-size: 13px; line-height: 1; color: var(--ink); transition: border-color 150ms var(--ease-ui), transform 160ms var(--ease-step); }
  .ghost.open { border-color: var(--faint); }
  @media (hover: hover) and (pointer: fine) { .ghost:hover { border-color: var(--faint); } }
  .ghost:active { transform: scale(0.97); }
  .adder .menu-item { font-size: 13.5px; line-height: 1.3; }
  .error { margin-top: 12px; font-size: 13px; line-height: 1.4; color: var(--text-red); overflow-wrap: anywhere; }
  .list { margin-top: 12px; border: 1px solid var(--rule); border-radius: 12px; }
  .item { display: flow-root; scroll-margin-bottom: 12px; }
  .item:not(:first-child) > .inner { border-top: 1px solid var(--rule); }
  @media (prefers-reduced-motion: reduce) { .ghost:active { transform: none; } }
</style>
