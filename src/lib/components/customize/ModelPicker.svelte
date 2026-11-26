<script lang="ts">
  import Check from 'lucide-svelte/icons/check';
  import Search from 'lucide-svelte/icons/search';
  import { onMount, tick } from 'svelte';
  import { isReady, modelsOf, type Job, type ProviderView } from '../../api/providers';
  import type { ProviderConfig } from '../../api/types';
  import { rise } from '../../motion/transitions';
  import { settings, updateSettings } from '../../state/settings.svelte';

  interface Props {
    job: Job;
    current: ProviderView;
    views: ProviderView[];
    onclose: () => void;
  }
  let { job, current, views, onclose }: Props = $props();

  interface Option {
    provider: ProviderConfig;
    /** '' only says that a name has to be typed. */
    model: string;
    text: string;
  }

  let query = $state('');
  let input = $state<HTMLInputElement>();
  let list = $state<HTMLElement>();

  const model = $derived(settings.value[job.modelKey]);
  const typed = $derived(query.trim());
  const choices = $derived([current, ...views.filter((v) => v !== current && isReady(v))]);

  const groups = $derived.by(() => {
    const q = typed.toLowerCase();
    const found = choices.map((view) => {
      const { provider } = view;
      const listed = modelsOf(view);
      const models = provider === current.provider && model && !listed.includes(model) ? [model, ...listed] : listed;
      const matches = provider.name.toLowerCase().includes(q)
        ? (models.length ? models : [''])
        : models.filter((m) => m.toLowerCase().includes(q));
      return {
        label: provider.name,
        options: matches.map((m): Option => ({ provider, model: m, text: m || 'Type the name of its model' })),
      };
    });
    const free = choices.filter(
      (v) => typed && isReady(v) && !found.some((g) => g.options.some((o) => o.provider === v.provider && o.model === typed)),
    );
    return [
      ...found,
      { label: `Use “${typed}” with`, options: free.map(({ provider }): Option => ({ provider, model: typed, text: provider.name })) },
    ].filter((g) => g.options.length > 0);
  });
  const options = $derived(groups.flatMap((g) => g.options));

  const isCurrent = (o: Option) => o.provider === current.provider && o.model === model;
  const named = (o: Option) => o.text !== o.provider.name && o.text.toLowerCase() === typed.toLowerCase();
  const start = () => Math.max(0, options.findIndex(typed ? named : isCurrent));

  let index = $state(start());

  const focusInput = () => input?.focus({ preventScroll: true });

  onMount(() => {
    focusInput();
    go(index);
  });

  function go(i: number) {
    if (!options.length) return;
    index = (i + options.length) % options.length;
    const el = list?.querySelectorAll<HTMLElement>('.opt')[index];
    if (!list || !el) return;
    if (index === 0) list.scrollTop = 0;
    else if (el.offsetTop < list.scrollTop) list.scrollTop = el.offsetTop;
    else list.scrollTop = Math.max(list.scrollTop, el.offsetTop + el.offsetHeight - list.clientHeight);
  }

  function pick(o: Option) {
    if (!o.model) return;
    if (!isCurrent(o)) void updateSettings({ [job.providerKey]: o.provider.id, [job.modelKey]: o.model });
    onclose();
  }

  async function oninput() {
    index = start();
    await tick();
    if (list) list.scrollTop = 0;
    go(index);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.isComposing || e.shiftKey || e.ctrlKey || e.altKey || e.metaKey || !options.length) return;
    if (e.key === 'ArrowDown') go(index + 1);
    else if (e.key === 'ArrowUp') go(index - 1);
    else if (e.key === 'Home') go(0);
    else if (e.key === 'End') go(options.length - 1);
    else if (e.key === 'Enter') pick(options[index]);
    else return;
    e.preventDefault();
  }
</script>

<div class="popover picker" role="dialog" aria-label="{job.name} model" tabindex="-1" onfocus={focusInput} transition:rise={{ y: -4, scale: 0.97, duration: 160 }}>
  <label class="find">
    <Search size={14} strokeWidth={1.5} />
    <input
      bind:this={input}
      bind:value={query}
      {oninput}
      {onkeydown}
      role="combobox"
      aria-expanded={options.length > 0}
      aria-controls="models-list"
      aria-activedescendant={options.length ? `model-${index}` : undefined}
      aria-label="Filter or type a model name"
      placeholder="Filter or type model…"
      spellcheck="false"
      autocomplete="off"
    />
  </label>
  <div class="opts" id="models-list" role="listbox" aria-label="Models" tabindex="-1" onfocus={focusInput} bind:this={list}>
    {#each groups as g}
      <div class="group" role="group" aria-label={g.label}>
        <div class="glabel" aria-hidden="true" title={g.label}>{g.label}</div>
        {#each g.options as o (`${o.provider.id} ${o.model}`)}
          {@const i = options.indexOf(o)}
          <div
            id="model-{i}"
            class="opt"
            class:active={i === index}
            class:current={isCurrent(o)}
            class:note={!o.model}
            role="option"
            aria-selected={i === index}
            aria-disabled={!o.model}
            tabindex="-1"
            title={o.text}
            onpointerdown={(e) => e.preventDefault()}
            onclick={() => pick(o)}
            onmousemove={() => (index = i)}
            onkeydown={() => {}}
          >
            <span class="what">{o.text}</span>
            {#if isCurrent(o)}<span class="check" role="img" aria-label="In use"><Check size={14} strokeWidth={1.6} /></span>{/if}
          </div>
        {/each}
      </div>
    {/each}
  </div>
</div>

<style>
  .picker {
    outline: none;
    top: calc(100% + 6px);
    right: 0;
    width: 280px;
    max-width: calc(100vw - 32px);
    padding: 4px;
    border-radius: 12px;
    transform-origin: top right;
  }
  .find {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    margin: -4px -4px 4px;
    padding: 0 12px;
    border-bottom: 1px solid var(--rule);
    color: var(--faint);
  }
  .find :global(svg) { flex: none; }
  .find input {
    flex: 1;
    min-width: 0;
    border: 0;
    padding: 0;
    background: transparent;
    outline: none;
    font: inherit;
    font-size: 13px;
    color: var(--ink);
  }
  .find input::placeholder { color: var(--faint); opacity: 1; }
  .opts {
    position: relative;
    outline: none;
    max-height: 248px;
    overflow-y: auto;
    overscroll-behavior: contain;
    scrollbar-width: thin;
    scrollbar-color: var(--rule) transparent;
  }
  @media (max-height: 699px) { .opts { max-height: 184px; } }
  .group + .group {
    margin-top: 4px;
    padding-top: 4px;
    border-top: 1px solid var(--rule);
  }
  .glabel {
    padding: 4px 10px 3px;
    font-size: 11.5px;
    line-height: 1.3;
    font-weight: 500;
    color: var(--faint);
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 10px;
    border-radius: 8px;
    font-size: 13px;
    line-height: 1.3;
    color: var(--graphite);
    cursor: default;
  }
  .opt.active {
    background: var(--riddle-wash);
    color: var(--ink);
  }
  .opt.current {
    color: var(--ink);
    font-weight: 500;
  }
  .opt.note { color: var(--faint); }
  .what { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .check { display: grid; flex: none; color: var(--ink); }
</style>
