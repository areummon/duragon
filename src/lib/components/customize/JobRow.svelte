<script lang="ts">
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import { accessProblem, jobProvider, THINKING_LEVELS, type Job, type ProviderView } from '../../api/providers';
  import { closeOnEscape } from '../../escape';
  import { settings } from '../../state/settings.svelte';
  import ModelPicker from './ModelPicker.svelte';
  import Reach from './Reach.svelte';

  let { job, views, onsetup }: { job: Job; views: ProviderView[]; onsetup: () => void } = $props();

  let open = $state(false);
  let root = $state<HTMLElement>();
  let button = $state<HTMLElement>();

  const current = $derived(jobProvider(views, settings.value, job));
  const model = $derived(settings.value[job.modelKey] || 'Choose a model');
  const problem = $derived(accessProblem(current));
  const thinking = $derived.by(() => {
    const level = THINKING_LEVELS.find((t) => t.id === settings.value[job.thinkingKey]);
    return level && level.id !== 'default' ? ` · ${level.name.toLowerCase()} thinking` : '';
  });

  const close = () => (open = false);

  $effect(() => {
    if (!open) return;
    const away = (e: Event) => {
      if (!root?.contains(e.target as Node)) close();
    };
    window.addEventListener('pointerdown', away, true);
    window.addEventListener('focusin', away);
    const off = closeOnEscape(close, () => button);
    return () => {
      window.removeEventListener('pointerdown', away, true);
      window.removeEventListener('focusin', away);
      off();
    };
  });
</script>

<div class="job" bind:this={root}>
  <button class="row" class:open bind:this={button} aria-haspopup="dialog" aria-expanded={open} onclick={() => (open = !open)}>
    <span class="text"><span class="name">{job.name}</span><span class="desc">{job.desc}</span></span>
    <span class="val">
      <span class="model" title={model}>{model}</span>
      {#if problem}
        <span class="by" title={problem}><span class="who"><i class="dot"></i>{problem}</span></span>
      {:else}
        <span class="by" title="{current.provider.name}{thinking}"><span class="who">{current.provider.name}</span><span class="think">{thinking}</span></span>
      {/if}
    </span>
    <span class="chev"><ChevronDown size={14} strokeWidth={1.7} /></span>
  </button>
  {#if open}
    <ModelPicker {job} {current} {views} onclose={close} {onsetup} />
    <Reach />
  {/if}
</div>

<style>
  .job { position: relative; }
  .job:not(:first-child) { border-top: 1px solid var(--rule); }
  .row {
    display: flex;
    align-items: baseline;
    gap: 24px;
    width: 100%;
    min-height: 66px;
    padding: 12px 14px 12px 16px;
    text-align: left;
    transition: background-color 150ms var(--ease-ui);
  }
  .row.open { background: var(--shelf); }
  @media (hover: hover) and (pointer: fine) { .row:hover { background: var(--shelf); } }
  .row:focus-visible { outline-offset: -4px; border-radius: 11px; }
  .job:first-child .row { border-top-left-radius: 11px; border-top-right-radius: 11px; }
  .job:last-child .row { border-bottom-left-radius: 11px; border-bottom-right-radius: 11px; }
  .text { flex: 1 1 0; min-width: 176px; }
  .name, .desc, .model { display: block; }
  .name { font-size: 13.5px; line-height: 1.3; font-weight: 500; }
  .desc { margin-top: 3px; font-size: 13px; line-height: 1.4; color: var(--faint); }
  .val { flex: 0 1 auto; min-width: 0; max-width: 250px; text-align: right; }
  .model, .who { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .model { font-size: 13.5px; line-height: 1.3; }
  .by { display: flex; justify-content: flex-end; margin-top: 3px; font-size: 13px; line-height: 1.4; color: var(--graphite); }
  .who { min-width: 0; }
  .think { flex: none; white-space: pre; }
  .dot { display: inline-block; width: 6px; height: 6px; margin: 0 7px 1px 0; border-radius: 50%; background: var(--bulb); }
  .chev { align-self: center; flex: none; margin-left: -14px; color: var(--faint); }
  .chev :global(svg) { display: block; }
</style>
