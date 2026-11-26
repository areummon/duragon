<script lang="ts">
  import CircleHelp from 'lucide-svelte/icons/circle-help';
  import Keyboard from 'lucide-svelte/icons/keyboard';
  import KeyRound from 'lucide-svelte/icons/key-round';
  import Palette from 'lucide-svelte/icons/palette';
  import SlidersHorizontal from 'lucide-svelte/icons/sliders-horizontal';
  import User from 'lucide-svelte/icons/user';
  import Users from 'lucide-svelte/icons/users';
  import Volume2 from 'lucide-svelte/icons/volume-2';
  import type { Component } from 'svelte';
  import { accessProblem, jobProvider, JOBS, providerViews } from '../../api/providers';
  import { rise } from '../../motion/transitions';
  import { app, type CustomizePage } from '../../state/app.svelte';
  import { credentialStatuses, searchStatus, servers, settings } from '../../state/settings.svelte';
  import Modal from '../Modal.svelte';
  import AboutPage from './AboutPage.svelte';
  import AppearancePage from './AppearancePage.svelte';
  import GeneralPage from './GeneralPage.svelte';
  import HelpPage from './HelpPage.svelte';
  import KeyboardPage from './KeyboardPage.svelte';
  import MindsPage from './MindsPage.svelte';
  import ProvidersPage from './ProvidersPage.svelte';
  import VoicePage from './VoicePage.svelte';

  type NavIcon = typeof SlidersHorizontal;

  interface NavItem {
    id: CustomizePage;
    label: string;
    title: string;
    icon: NavIcon;
    component: Component;
    gap?: 'group' | 'bottom';
  }

  const NAV: NavItem[] = [
    { id: 'general', label: 'General', title: 'General', icon: SlidersHorizontal, component: GeneralPage },
    { id: 'appearance', label: 'Appearance', title: 'Appearance', icon: Palette, component: AppearancePage },
    { id: 'keyboard', label: 'Keyboard', title: 'Keyboard', icon: Keyboard, component: KeyboardPage },
    { id: 'providers', label: 'Providers', title: 'Providers', icon: KeyRound, component: ProvidersPage, gap: 'group' },
    { id: 'minds', label: 'Minds', title: 'Minds', icon: Users, component: MindsPage },
    { id: 'about', label: 'About you', title: 'About you', icon: User, component: AboutPage },
    { id: 'voice', label: 'Voice', title: 'Voice', icon: Volume2, component: VoicePage },
    { id: 'help', label: 'Help', title: 'Help', icon: CircleHelp, component: HelpPage, gap: 'bottom' },
  ];

  let scrollEl = $state<HTMLElement>();
  const current = $derived(NAV.find((n) => n.id === app.customizePage) ?? NAV[0]);
  const views = $derived(
    app.customizeOpen
      ? providerViews(settings.value, credentialStatuses.value, credentialStatuses.error !== null, servers)
      : [],
  );
  const providersAttention = $derived(
    app.customizeOpen &&
      (JOBS.some((job) => accessProblem(jobProvider(views, settings.value, job)) !== null) ||
        !!(searchStatus.value?.pinned && !searchStatus.value.available)),
  );

  const close = () => (app.customizeOpen = false);

  $effect(() => {
    void app.customizePage;
    if (scrollEl) scrollEl.scrollTop = 0;
  });
</script>

<Modal open={app.customizeOpen} onclose={close} label="Customize Duragon" class="customize-card">
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
    <div class="scroll" bind:this={scrollEl}>
      <h2>{current.title}</h2>
      {#key app.customizePage}
        <div class="page" in:rise={{ y: 4, duration: 180 }}>
          <current.component />
        </div>
      {/key}
    </div>
  </div>
</Modal>

<style>
  :global(.card.customize-card) {
    display: flex;
    width: min(860px, 100%);
    height: min(720px, 100%);
    overflow: hidden;
  }
  :global(.card.customize-card > .x) {
    top: 28px;
    right: 28px;
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
  .scroll { height: 100%; overflow-y: auto; overscroll-behavior: contain; padding: 34px 40px 40px 40px; scrollbar-width: thin; scrollbar-color: var(--rule) transparent; }
  h2 { margin: 0; font-size: 19px; line-height: 1.3; font-weight: 500; letter-spacing: -0.01em; }
  .page { margin-top: 22px; }
  @media (max-width: 700px) {
    nav { width: 156px; }
    .scroll { padding: 28px 20px; }
  }
</style>
