<script lang="ts">
  import { Search, X } from 'lucide-svelte';
  import { tick } from 'svelte';
  import { pop, rise, veil } from '../../motion/transitions';
  import { app, type CustomizePage } from '../../state/app.svelte';
  import AboutPage from './AboutPage.svelte';
  import CustomizePageView from './CustomizePage.svelte';
  import HelpPage from './HelpPage.svelte';
  import MindsPage from './MindsPage.svelte';
  import VoiceModelsPage from './VoiceModelsPage.svelte';

  const NAV: { id: CustomizePage; label: string; title: string }[] = [
    { id: 'customize', label: 'Customize Duragon', title: 'Customize Duragon' },
    { id: 'minds', label: 'Minds', title: 'Minds' },
    { id: 'about', label: 'About you', title: 'About you' },
    { id: 'voice', label: 'Voice models', title: 'Voice models' },
    { id: 'help', label: 'Help', title: 'Help' },
  ];

  let query = $state('');
  let card = $state<HTMLElement>();
  const current = $derived(NAV.find((n) => n.id === app.customizePage) ?? NAV[0]);

  const close = () => (app.customizeOpen = false);

  $effect(() => {
    if (app.customizeOpen) void tick().then(() => card?.focus());
    else query = '';
  });

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && app.customizeOpen && !app.mindEditor) {
      e.preventDefault();
      close();
    }
  }
</script>

<svelte:window {onkeydown} />

{#if app.customizeOpen}
  <div class="scrim" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()} transition:veil>
    <div class="card" bind:this={card} tabindex="-1" role="dialog" aria-modal="true" aria-label="Customize Duragon" in:pop out:pop={{ duration: 140 }}>
      <nav aria-label="Settings sections">
        {#each NAV as n (n.id)}
          <button class:active={app.customizePage === n.id} aria-current={app.customizePage === n.id ? 'page' : undefined} onclick={() => (app.customizePage = n.id)}>{n.label}</button>
        {/each}
      </nav>
      <div class="main">
        <button class="x" aria-label="Close" onclick={close}><X size={17} strokeWidth={1.5} /></button>
        <div class="scroll">
          <h2>{current.title}</h2>
          {#key app.customizePage}
            <div class="page" in:rise={{ y: 4, duration: 180 }}>
              {#if app.customizePage === 'customize'}
                <label class="search">
                  <Search size={15} strokeWidth={1.5} />
                  <input type="search" placeholder="Search settings" aria-label="Search settings" bind:value={query} />
                </label>
                <CustomizePageView {query} />
              {:else if app.customizePage === 'minds'}
                <MindsPage />
              {:else if app.customizePage === 'about'}
                <AboutPage />
              {:else if app.customizePage === 'voice'}
                <VoiceModelsPage />
              {:else}
                <HelpPage />
              {/if}
            </div>
          {/key}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(0, 0, 0, 0.45);
  }
  :global([data-theme='light']) .scrim { background: rgba(29, 28, 26, 0.2); }
  .card {
    display: flex;
    width: min(860px, 100%);
    height: min(720px, 100%);
    overflow: hidden;
    border-radius: 14px;
    background: var(--paper);
    box-shadow: var(--lift);
    font-size: 14px;
    line-height: 1.4;
    outline: none;
  }
  nav {
    flex: none;
    width: 220px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 18px 10px;
    border-right: 1px solid var(--rule);
  }
  nav button {
    height: 38px;
    padding: 0 13px;
    border: 1px solid transparent;
    border-radius: 9px;
    text-align: left;
    font-size: 14px;
    color: var(--graphite);
    transition: color 150ms var(--ease-ui), background-color 150ms var(--ease-ui), border-color 150ms var(--ease-ui);
  }
  nav button:hover { color: var(--ink); }
  nav button.active { color: var(--ink); background: var(--riddle-wash); border-color: var(--rule); }
  .main { position: relative; flex: 1; min-width: 0; }
  .x { position: absolute; top: 24px; right: 22px; z-index: 2; display: grid; place-items: center; width: 28px; height: 28px; color: var(--graphite); }
  .x:hover { color: var(--ink); }
  .scroll { height: 100%; overflow-y: auto; padding: 36px 52px 36px 36px; scrollbar-width: thin; scrollbar-color: var(--rule) transparent; }
  h2 { margin: 0; font-size: 19px; line-height: 1.3; font-weight: 500; }
  .page { margin-top: 20px; }
  .search {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 42px;
    padding: 0 14px;
    border: 1px solid var(--rule);
    border-radius: 12px;
    color: var(--faint);
  }
  .search:focus-within { border-color: var(--faint); }
  .search input { flex: 1; min-width: 0; border: 0; background: transparent; outline: none; font: inherit; font-size: 14px; color: var(--ink); }
  .search input::placeholder { color: var(--faint); }
  .search input::-webkit-search-cancel-button { display: none; }
  @media (max-width: 700px) {
    nav { width: 150px; }
    .scroll { padding: 28px 20px; }
  }
</style>
