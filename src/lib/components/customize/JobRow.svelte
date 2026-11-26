<script lang="ts">
  import Check from 'lucide-svelte/icons/check';
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import { accessProblem, jobProvider, THINKING_LEVELS, type Job, type ProviderView } from '../../api/providers';
  import { closeOnEscape } from '../../escape';
  import { rise } from '../../motion/transitions';
  import { settings, updateSettings } from '../../state/settings.svelte';
  import ModelPicker from './ModelPicker.svelte';
  import Reach from './Reach.svelte';

  type Thinking = (typeof THINKING_LEVELS)[number]['id'];

  let { job, views }: { job: Job; views: ProviderView[] } = $props();

  let open = $state<'model' | 'thinking' | null>(null);
  let root = $state<HTMLElement>();
  let modelBtn = $state<HTMLElement>();
  let thinkBtn = $state<HTMLElement>();

  const current = $derived(jobProvider(views, settings.value, job));
  const model = $derived(settings.value[job.modelKey] || 'Choose model');
  const problem = $derived(accessProblem(current));
  const thinkingLevel = $derived(
    THINKING_LEVELS.find((t) => t.id === settings.value[job.thinkingKey]) ?? THINKING_LEVELS[0],
  );

  const close = () => (open = null);
  const toggle = (which: 'model' | 'thinking') => {
    open = open === which ? null : which;
  };

  function pickThinking(id: Thinking) {
    if (settings.value[job.thinkingKey] !== id) {
      void updateSettings({ [job.thinkingKey]: id });
    }
    close();
  }

  $effect(() => {
    if (!open) return;
    const anchor = open === 'model' ? () => modelBtn : () => thinkBtn;
    const away = (e: Event) => {
      if (!root?.contains(e.target as Node)) close();
    };
    window.addEventListener('pointerdown', away, true);
    window.addEventListener('focusin', away);
    const off = closeOnEscape(close, anchor);
    return () => {
      window.removeEventListener('pointerdown', away, true);
      window.removeEventListener('focusin', away);
      off();
    };
  });
</script>

<div class="job" bind:this={root}>
  <div class="row">
    <div class="text">
      <div class="name-line">
        <span class="name">{job.name}</span>
        {#if problem}
          <span class="provider warn" title={problem}><i class="dot" aria-hidden="true"></i>{problem}</span>
        {:else}
          <span class="provider" title={current.provider.name}>{current.provider.name}</span>
        {/if}
      </div>
      <span class="desc">{job.desc}</span>
    </div>

    <div class="ctrl">
      <button
        type="button"
        class="select"
        class:open={open === 'model'}
        bind:this={modelBtn}
        aria-haspopup="dialog"
        aria-expanded={open === 'model'}
        aria-label="{job.name} model: {model}"
        title="{model} ({current.provider.name})"
        onclick={() => toggle('model')}
      >
        <span class="select-val">{model}</span>
        <ChevronDown class="chev" size={13} strokeWidth={1.7} />
      </button>
      {#if open === 'model'}
        <ModelPicker {job} {current} {views} onclose={close} />
        <Reach />
      {/if}
    </div>

    <div class="ctrl">
      <button
        type="button"
        class="select"
        class:open={open === 'thinking'}
        bind:this={thinkBtn}
        aria-haspopup="listbox"
        aria-expanded={open === 'thinking'}
        aria-label="{job.name} thinking: {thinkingLevel.name}"
        title="Thinking: {thinkingLevel.name}"
        onclick={() => toggle('thinking')}
      >
        <span class="select-val">{thinkingLevel.name}</span>
        <ChevronDown class="chev" size={13} strokeWidth={1.7} />
      </button>
      {#if open === 'thinking'}
        <div
          class="popover think-menu"
          role="listbox"
          aria-label="{job.name} thinking level"
          transition:rise={{ y: -4, scale: 0.97, duration: 150 }}
        >
          {#each THINKING_LEVELS as level (level.id)}
            {@const active = level.id === thinkingLevel.id}
            <button
              type="button"
              class="think-opt"
              class:active
              role="option"
              aria-selected={active}
              onclick={() => pickThinking(level.id)}
            >
              <span>{level.name}</span>
              {#if active}<Check size={14} strokeWidth={1.6} />{/if}
            </button>
          {/each}
        </div>
        <Reach />
      {/if}
    </div>
  </div>
</div>

<style>
  .job { position: relative; }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 172px 104px;
    align-items: center;
    gap: 10px;
    min-height: 60px;
    padding: 10px 14px 10px 16px;
  }
  .text { min-width: 0; padding-right: 6px; }
  .name-line {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .name {
    flex: none;
    font-size: 13.5px;
    line-height: 1.3;
    font-weight: 500;
    color: var(--ink);
  }
  .provider {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    line-height: 1.3;
    color: var(--faint);
  }
  .provider.warn {
    color: var(--graphite);
  }
  .dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    margin: 0 5px 1px 0;
    border-radius: 50%;
    background: var(--bulb);
  }
  .desc {
    display: block;
    margin-top: 2px;
    font-size: 12.5px;
    line-height: 1.4;
    color: var(--faint);
  }
  .ctrl {
    position: relative;
    min-width: 0;
  }
  .select {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    height: 32px;
    padding: 0 9px 0 11px;
    border: 1px solid var(--rule);
    border-radius: 8px;
    background: var(--paper);
    color: var(--ink);
    font-size: 13px;
    line-height: 1.3;
    text-align: left;
    transition:
      border-color var(--dur-ui) var(--ease-ui),
      background-color var(--dur-ui) var(--ease-ui);
  }
  @media (hover: hover) and (pointer: fine) {
    .select:hover {
      background: var(--shelf);
      border-color: color-mix(in srgb, var(--ink) 18%, var(--rule));
    }
  }
  .select.open {
    background: var(--shelf);
    border-color: color-mix(in srgb, var(--ink) 26%, var(--rule));
  }
  .select-val {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .select :global(.chev) {
    flex: none;
    color: var(--faint);
    transition: transform 160ms var(--ease-ui), color var(--dur-ui) var(--ease-ui);
  }
  .select.open :global(.chev) {
    color: var(--ink);
    transform: rotate(180deg);
  }
  .think-menu {
    top: calc(100% + 6px);
    right: 0;
    width: 128px;
    padding: 4px;
    border-radius: 12px;
    transform-origin: top right;
  }
  .think-opt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 10px;
    border-radius: 8px;
    font-size: 13px;
    line-height: 1.3;
    color: var(--graphite);
    text-align: left;
    transition: background-color 120ms var(--ease-ui), color 120ms var(--ease-ui);
  }
  @media (hover: hover) and (pointer: fine) {
    .think-opt:hover {
      background: var(--riddle-wash);
      color: var(--ink);
    }
  }
  .think-opt.active {
    color: var(--ink);
    font-weight: 500;
  }
  @media (max-width: 680px) {
    .row {
      grid-template-columns: minmax(0, 1fr) 104px;
      gap: 8px;
    }
    .text {
      grid-column: 1 / -1;
      padding-right: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .select :global(.chev) {
      transition: none;
    }
  }
</style>
