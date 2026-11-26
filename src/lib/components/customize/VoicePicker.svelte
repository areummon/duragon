<script lang="ts">
  import Check from 'lucide-svelte/icons/check';
  import Pause from 'lucide-svelte/icons/pause';
  import Play from 'lucide-svelte/icons/play';
  import Search from 'lucide-svelte/icons/search';
  import { onMount, tick } from 'svelte';
  import type { VoiceOption } from '../../api/types';
  import { rise } from '../../motion/transitions';
  import { samplePlaying, toggleSample } from '../../state/voice.svelte';

  interface Props {
    /** Names the dialog, and prefixes its element ids. */
    label: string;
    id: string;
    placeholder: string;
    options: VoiceOption[];
    current: string | undefined;
    describe: (o: VoiceOption) => string;
    /** Voices get a button that plays a sample. */
    samples?: boolean;
    empty: string;
    more: string;
    onpick: (id: string) => void;
    onmore: () => void;
    onclose: () => void;
  }
  let { label, id, placeholder, options, current, describe, samples = false, empty, more, onpick, onmore, onclose }: Props = $props();

  let query = $state('');
  let input = $state<HTMLInputElement>();
  let list = $state<HTMLElement>();

  const q = $derived(query.trim().toLowerCase());
  const groups = $derived.by(() => {
    const out: { label: string; options: VoiceOption[] }[] = [];
    for (const o of options) {
      if (q && !`${o.name} ${describe(o)} ${o.engine}`.toLowerCase().includes(q)) continue;
      const name = `${o.engine} · ${o.source}`;
      const group = out.find((g) => g.label === name);
      if (group) group.options.push(o);
      else out.push({ label: name, options: [o] });
    }
    return out;
  });
  const shown = $derived(groups.flatMap((g) => g.options));
  const start = () => (q ? 0 : Math.max(0, shown.findIndex((o) => o.id === current)));

  /** Kept by id, so the same option stays highlighted while the list changes under it. */
  let active = $state<string>();
  const index = $derived(Math.max(0, shown.findIndex((o) => o.id === active)));

  const focusInput = () => input?.focus({ preventScroll: true });

  onMount(() => {
    focusInput();
    go(start());
  });

  function go(i: number) {
    if (!shown.length) return;
    const n = (i + shown.length) % shown.length;
    active = shown[n].id;
    const el = list?.querySelectorAll<HTMLElement>('.opt')[n];
    if (!list || !el) return;
    if (n === 0) list.scrollTop = 0;
    else if (el.offsetTop < list.scrollTop) list.scrollTop = el.offsetTop;
    else list.scrollTop = Math.max(list.scrollTop, el.offsetTop + el.offsetHeight - list.clientHeight);
  }

  function pick(o: VoiceOption) {
    if (o.id !== current) onpick(o.id);
    onclose();
  }

  async function oninput() {
    await tick();
    if (list) list.scrollTop = 0;
    go(start());
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.isComposing || e.shiftKey || e.ctrlKey || e.altKey || e.metaKey || !shown.length) return;
    if (e.key === 'ArrowDown') go(index + 1);
    else if (e.key === 'ArrowUp') go(index - 1);
    else if (e.key === 'Home') go(0);
    else if (e.key === 'End') go(shown.length - 1);
    else if (e.key === 'Enter') pick(shown[index]);
    else return;
    e.preventDefault();
  }
</script>

<div class="popover picker" role="dialog" aria-label={label} tabindex="-1" onfocus={focusInput} transition:rise={{ y: -4, scale: 0.97, duration: 180 }}>
  <label class="find">
    <Search size={15} strokeWidth={1.5} />
    <input
      bind:this={input}
      bind:value={query}
      {oninput}
      {onkeydown}
      role="combobox"
      aria-expanded={shown.length > 0}
      aria-controls="{id}-list"
      aria-activedescendant={shown.length ? `${id}-${index}` : undefined}
      aria-label={placeholder}
      {placeholder}
      spellcheck="false"
      autocomplete="off"
    />
  </label>
  {#if !options.length}
    <p class="empty">{empty}</p>
  {:else}
    <div class="opts" id="{id}-list" role="listbox" aria-label={label} tabindex="-1" onfocus={focusInput} bind:this={list}>
      {#each groups as g (g.label)}
        <div role="group" aria-label={g.label}>
          <div class="glabel" aria-hidden="true" title={g.label}>{g.label}</div>
          {#each g.options as o (o.id)}
            {@const i = shown.indexOf(o)}
            {@const playing = samples && samplePlaying(o.id)}
            <div id="{id}-{i}" class="opt" class:active={i === index} role="option" aria-selected={i === index} tabindex="-1" title="{o.name} · {describe(o)}"
              onpointerdown={(e) => e.preventDefault()} onclick={() => pick(o)} onmousemove={() => (active = o.id)} onkeydown={() => {}}>
              <span class="what">{o.name}</span>
              <span class="desc">{describe(o)}</span>
              <span class="check">{#if o.id === current}<span role="img" aria-label="In use"><Check size={15} strokeWidth={1.6} /></span>{/if}</span>
              {#if samples}
                <button class="play" class:on={playing} class:show={i === index || playing} tabindex="-1"
                  aria-label="{playing ? 'Stop the sample of' : 'Play a sample of'} {o.name}"
                  onclick={(e) => { e.stopPropagation(); toggleSample(o.id); }}>
                  {#if playing}<Pause size={12} strokeWidth={1.8} />{:else}<Play size={12} strokeWidth={1.8} />{/if}
                </button>
              {/if}
            </div>
          {/each}
        </div>
      {/each}
    </div>
  {/if}
  <div class="more"><button class="menu-item" onclick={onmore}>{more}</button></div>
</div>

<style>
  .picker { outline: none; top: calc(100% - 6px); right: 8px; width: 340px; max-width: calc(100% - 16px); transform-origin: top right; }
  .find { display: flex; align-items: center; gap: 9px; height: 40px; margin: -4px -4px 4px; padding: 0 14px; border-bottom: 1px solid var(--rule); color: var(--faint); }
  .find :global(svg) { flex: none; }
  .find input { flex: 1; min-width: 0; border: 0; padding: 0; background: transparent; outline: none; font: inherit; font-size: 13.5px; color: var(--ink); }
  .find input::placeholder { color: var(--faint); opacity: 1; }
  .opts { position: relative; outline: none; max-height: 232px; overflow-y: auto; overscroll-behavior: contain; scrollbar-width: thin; scrollbar-color: var(--rule) transparent; }
  @media (max-height: 699px) { .opts { max-height: 152px; } }
  .opt { display: flex; align-items: center; gap: 8px; min-height: 32px; padding: 4px 6px 4px 10px; border-radius: 7px; font-size: 13.5px; line-height: 1.3; cursor: default; }
  .opt.active { background: var(--shelf); }
  .what { flex: none; max-width: 60%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .desc { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 13px; color: var(--faint); }
  .check { display: grid; place-items: center; flex: none; width: 15px; color: var(--graphite); }
  .check :global(svg) { display: block; }
  .play { display: grid; place-items: center; flex: none; width: 24px; height: 24px; border-radius: 50%; color: var(--graphite); opacity: 0; pointer-events: none; transition: opacity 150ms var(--ease-ui), background-color 150ms var(--ease-ui); }
  .play.show { opacity: 1; pointer-events: auto; }
  .play.on, .play:hover { background: var(--riddle-wash); color: var(--ink); }
  .play :global(svg) { display: block; }
  .empty { padding: 10px 10px 12px; font-size: 13.5px; line-height: 1.4; color: var(--faint); }
  .more { margin: 4px -4px -4px; padding: 4px; border-top: 1px solid var(--rule); }
  .more .menu-item { font-size: 13.5px; line-height: 1.3; color: var(--graphite); }
</style>
