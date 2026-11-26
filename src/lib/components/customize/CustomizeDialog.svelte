<script lang="ts">
  import CircleHelp from 'lucide-svelte/icons/circle-help';
  import Keyboard from 'lucide-svelte/icons/keyboard';
  import KeyRound from 'lucide-svelte/icons/key-round';
  import Palette from 'lucide-svelte/icons/palette';
  import SlidersHorizontal from 'lucide-svelte/icons/sliders-horizontal';
  import User from 'lucide-svelte/icons/user';
  import Users from 'lucide-svelte/icons/users';
  import Volume2 from 'lucide-svelte/icons/volume-2';
  import X from 'lucide-svelte/icons/x';
  import { tick } from 'svelte';
  import { accessProblem, jobProvider, JOBS, providerViews } from '../../api/providers';
  import { closeOnEscape } from '../../escape';
  import { pop, rise, veil } from '../../motion/transitions';
  import { app, type CustomizePage } from '../../state/app.svelte';
  import { credentialStatuses, searchStatus, servers, settings } from '../../state/settings.svelte';
  import AboutPage from './AboutPage.svelte';
  import AppearancePage from './AppearancePage.svelte';
  import GeneralPage from './GeneralPage.svelte';
  import HelpPage from './HelpPage.svelte';
  import KeyboardPage from './KeyboardPage.svelte';
  import MindsPage from './MindsPage.svelte';
  import ProvidersPage from './ProvidersPage.svelte';
  import VoicePage from './VoicePage.svelte';

  type NavIcon = typeof SlidersHorizontal;

  const NAV: { id: CustomizePage; label: string; title: string; icon: NavIcon; gap?: 'group' | 'bottom' }[] = [
    { id: 'general', label: 'General', title: 'General', icon: SlidersHorizontal },
    { id: 'appearance', label: 'Appearance', title: 'Appearance', icon: Palette },
    { id: 'keyboard', label: 'Keyboard', title: 'Keyboard', icon: Keyboard },
    { id: 'providers', label: 'Providers', title: 'Providers', icon: KeyRound, gap: 'group' },
    { id: 'minds', label: 'Minds', title: 'Minds', icon: Users },
    { id: 'about', label: 'About you', title: 'About you', icon: User },
    { id: 'voice', label: 'Voice', title: 'Voice', icon: Volume2 },
    { id: 'help', label: 'Help', title: 'Help', icon: CircleHelp, gap: 'bottom' },
  ];

  let card = $state<HTMLElement>();
  let scrollEl = $state<HTMLElement>();
  const current = $derived(NAV.find((n) => n.id === app.customizePage) ?? NAV[0]);
  const views = $derived(providerViews(settings.value, credentialStatuses.value, credentialStatuses.error !== null, servers));
  const providersAttention = $derived(
    JOBS.some((job) => accessProblem(jobProvider(views, settings.value, job)) !== null) ||
      !!(searchStatus.value?.pinned && !searchStatus.value.available),
  );

  const close = () => (app.customizeOpen = false);

  $effect(() => {
    if (app.customizeOpen) void tick().then(() => card?.focus());
  });

  $effect(() => {
    void app.customizePage;
    if (scrollEl) scrollEl.scrollTop = 0;
  });

  $effect(() => {
    if (app.customizeOpen) return closeOnEscape(close);
  });
</script>

{#if app.customizeOpen}
  <div class="scrim" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()} transition:veil|global>
    <div class="card" bind:this={card} tabindex="-1" role="dialog" aria-modal="true" aria-label="Customize Duragon" in:pop|global out:pop={{ duration: 140 }}>
      <nav aria-label="Settings sections">
        {#each NAV as n (n.id)}
          {@const Icon = n.icon}
          <button
            class:active={app.customizePage === n.id}
            class:gap-group={n.gap === 'group'}
            class:gap-bottom={n.gap === 'bottom'}
            aria-current={app.customizePage === n.id ? 'page' : undefined}
            onclick={() => (app.customizePage = n.id)}
          >
            <span class="nav-icon" aria-hidden="true"><Icon size={15} strokeWidth={1.6} /></span>
            <span class="nav-label">{n.label}</span>
            {#if n.id === 'providers' && providersAttention}<i class="nav-dot" aria-hidden="true"></i>{/if}
          </button>
        {/each}
      </nav>
      <div class="main">
        <button class="x" aria-label="Close" onclick={close}><X size={17} strokeWidth={1.5} /></button>
        <div class="scroll" bind:this={scrollEl}>
          <h2>{current.title}</h2>
          {#key app.customizePage}
            <div class="page" in:rise={{ y: 4, duration: 180 }}>
              {#if app.customizePage === 'general'}
                <GeneralPage />
              {:else if app.customizePage === 'appearance'}
                <AppearancePage />
              {:else if app.customizePage === 'providers'}
                <ProvidersPage />
              {:else if app.customizePage === 'keyboard'}
                <KeyboardPage />
              {:else if app.customizePage === 'minds'}
                <MindsPage />
              {:else if app.customizePage === 'about'}
                <AboutPage />
              {:else if app.customizePage === 'voice'}
                <VoicePage />
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
    background: var(--veil);
  }
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
    width: 216px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 20px 10px 14px;
    background: var(--shelf);
    border-right: 1px solid var(--rule);
  }
  nav button {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 11px;
    border-radius: 8px;
    text-align: left;
    font-size: 13.5px;
    line-height: 1.3;
    color: var(--graphite);
    transition: color 150ms var(--ease-ui), background-color 150ms var(--ease-ui);
  }
  nav button.gap-group {
    margin-top: 12px;
  }
  nav button.gap-bottom {
    margin-top: auto;
  }
  .nav-icon {
    flex: none;
    display: grid;
    place-items: center;
    color: var(--faint);
    transition: color 150ms var(--ease-ui);
  }
  .nav-label {
    flex: 1;
    text-align: left;
  }
  .nav-dot {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--bulb);
  }
  @media (hover: hover) and (pointer: fine) {
    nav button:hover { color: var(--ink); background: var(--riddle-wash); }
    nav button:hover .nav-icon { color: var(--ink); }
  }
  nav button.active { color: var(--ink); font-weight: 500; background: var(--riddle-wash); }
  nav button.active .nav-icon { color: var(--ink); }
  .main { position: relative; flex: 1; min-width: 0; }
  .x {
    position: absolute;
    top: 28px;
    right: 28px;
    z-index: 2;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    color: var(--faint);
    transition: color 150ms var(--ease-ui), background-color 150ms var(--ease-ui);
  }
  @media (hover: hover) and (pointer: fine) {
    .x:hover { color: var(--ink); background: var(--shelf); }
  }
  .scroll { height: 100%; overflow-y: auto; overscroll-behavior: contain; padding: 34px 40px 40px 40px; scrollbar-width: thin; scrollbar-color: var(--rule) transparent; }
  h2 { margin: 0; font-size: 19px; line-height: 1.3; font-weight: 500; letter-spacing: -0.01em; }
  .page { margin-top: 22px; }
  @media (max-width: 700px) {
    nav { width: 156px; }
    .scroll { padding: 28px 20px; }
  }
</style>
