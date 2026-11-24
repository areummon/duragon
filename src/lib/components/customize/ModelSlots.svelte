<script lang="ts">
  import Check from 'lucide-svelte/icons/check';
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import { isHttpKind } from '../../api/providers';
  import type { ProviderConfig, Settings, ThinkingLevel } from '../../api/types';
  import { credentialStatuses, settings, updateSettings } from '../../state/settings.svelte';

  interface Props {
    showRead: boolean;
    showChat: boolean;
    showDraw: boolean;
  }
  let { showRead, showChat, showDraw }: Props = $props();

  const s = $derived(settings.value);
  const set = (patch: Partial<Settings>) => void updateSettings(patch);

  const THINKING_LEVELS: readonly { id: ThinkingLevel; label: string; bars: number }[] = [
    { id: 'default', label: 'Auto', bars: 0 },
    { id: 'low', label: 'Low', bars: 1 },
    { id: 'medium', label: 'Med', bars: 2 },
    { id: 'high', label: 'High', bars: 3 },
    { id: 'xhigh', label: 'XHigh', bars: 4 },
  ];

  const SLOTS = [
    {
      id: 'read',
      providerKey: 'readProvider',
      modelKey: 'readModel',
      thinkingKey: 'readThinking',
      name: 'Reading',
      desc: 'Reads along quietly and asks questions in the margin.',
    },
    {
      id: 'chat',
      providerKey: 'chatProvider',
      modelKey: 'chatModel',
      thinkingKey: 'chatThinking',
      name: 'Chat',
      desc: 'Answers in the side chat and when you address it.',
    },
    {
      id: 'draw',
      providerKey: 'drawProvider',
      modelKey: 'drawModel',
      thinkingKey: 'drawThinking',
      name: 'Drawing',
      desc: 'Sketches diagrams beside your notes.',
    },
  ] as const;

  type Slot = (typeof SLOTS)[number];

  let openMenu = $state<string | null>(null);
  let drafts = $state<Record<string, string>>({});

  const visibleSlots = $derived(
    SLOTS.filter((slot) =>
      slot.id === 'read' ? showRead : slot.id === 'chat' ? showChat : showDraw,
    ),
  );

  function providerOf(id: string): ProviderConfig | undefined {
    return s.providers.find((p) => p.id === id);
  }

  function providerReady(id: string): boolean {
    const p = providerOf(id);
    if (!p) return false;
    if (!isHttpKind(p.kind)) return true;
    const st = credentialStatuses.value.find((c) => c.providerId === id);
    return !!(st?.hasStoredKey || st?.hasEnvKey);
  }

  function thinkingIndex(level: ThinkingLevel): number {
    const i = THINKING_LEVELS.findIndex((t) => t.id === level);
    return i >= 0 ? i : 0;
  }

  function thinkingBars(level: ThinkingLevel): number {
    return THINKING_LEVELS.find((t) => t.id === level)?.bars ?? 0;
  }

  function filteredModels(providerId: string, slot: Slot): string[] {
    const list = providerOf(providerId)?.models ?? [];
    const q = (drafts[slot.id] ?? '').trim().toLowerCase();
    if (!q || q === s[slot.modelKey].toLowerCase()) return list;
    return list.filter((m) => m.toLowerCase().includes(q));
  }

  function pickProvider(slot: Slot, id: string) {
    if (id === s[slot.providerKey]) return;
    const suggested = providerOf(id)?.models ?? [];
    const patch: Partial<Settings> = { [slot.providerKey]: id };
    if (suggested.length > 0) {
      if (!suggested.includes(s[slot.modelKey])) {
        patch[slot.modelKey] = suggested[0];
      }
    } else {
      patch[slot.modelKey] = '';
    }
    delete drafts[slot.id];
    openMenu = null;
    set(patch);
  }

  function commitModel(slot: Slot, raw: string) {
    const v = raw.trim();
    delete drafts[slot.id];
    openMenu = null;
    if (v && v !== s[slot.modelKey]) {
      set({ [slot.modelKey]: v });
    }
  }

  function onWindowMouseDown(e: MouseEvent) {
    if (!openMenu) return;
    const target = e.target as HTMLElement | null;
    if (!target?.closest(`.combo[data-slot="${openMenu}"]`)) {
      openMenu = null;
    }
  }
</script>

<svelte:window onmousedown={onWindowMouseDown} />

{#if visibleSlots.length > 0}
  <div class="slots">
    {#each visibleSlots as slot (slot.id)}
      {@const activeProvider = s[slot.providerKey]}
      {@const activeModel = s[slot.modelKey]}
      {@const activeThinking = s[slot.thinkingKey]}
      {@const ready = providerReady(activeProvider)}
      {@const suggestions = filteredModels(activeProvider, slot)}
      {@const isMenuOpen = openMenu === slot.id && suggestions.length > 0}
      {@const tIdx = thinkingIndex(activeThinking)}

      <div class="slot-row">
        <div class="slot-top">
          <div class="slot-copy">
            <h4>{slot.name}</h4>
            <p class="desc">{slot.desc}</p>
          </div>

          <div class="think" title="Thinking effort">
            <span class="think-meter" aria-hidden="true">
              {#each [1, 2, 3, 4] as n (n)}
                <i class:lit={thinkingBars(activeThinking) >= n} style:height="{4 + n * 2}px"></i>
              {/each}
            </span>
            <div class="seg" role="radiogroup" aria-label="{slot.name} thinking level">
              <span class="seg-thumb" aria-hidden="true" style:transform="translateX({tIdx * 100}%)"></span>
              {#each THINKING_LEVELS as t (t.id)}
                <button
                  class="seg-btn"
                  class:selected={activeThinking === t.id}
                  role="radio"
                  aria-checked={activeThinking === t.id}
                  onclick={() => set({ [slot.thinkingKey]: t.id })}
                >{t.label}</button>
              {/each}
            </div>
          </div>
        </div>

        <div class="controls">
          <div class="select-wrap">
            <span class="dot" class:ready aria-hidden="true" title={ready ? 'Ready' : 'API key needed'}></span>
            <select
              class="control select"
              aria-label="{slot.name} provider"
              value={activeProvider}
              onchange={(e) => pickProvider(slot, e.currentTarget.value)}
            >
              {#each s.providers as p (p.id)}
                <option value={p.id}>{p.name}</option>
              {/each}
            </select>
            <span class="chevron" aria-hidden="true"><ChevronDown size={14} strokeWidth={1.7} /></span>
          </div>

          <div class="combo" data-slot={slot.id}>
            <input
              class="control model-input"
              class:has-list={(providerOf(activeProvider)?.models.length ?? 0) > 0}
              value={drafts[slot.id] ?? activeModel}
              aria-label="{slot.name} model"
              aria-expanded={isMenuOpen}
              autocomplete="off"
              spellcheck="false"
              placeholder="Model name"
              onfocus={() => {
                drafts[slot.id] = activeModel;
                openMenu = slot.id;
              }}
              oninput={(e) => {
                drafts[slot.id] = e.currentTarget.value;
                openMenu = slot.id;
              }}
              onkeydown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  commitModel(slot, drafts[slot.id] ?? activeModel);
                  (e.currentTarget as HTMLInputElement).blur();
                } else if (e.key === 'Escape') {
                  delete drafts[slot.id];
                  openMenu = null;
                  (e.currentTarget as HTMLInputElement).blur();
                }
              }}
              onblur={() => {
                const v = drafts[slot.id] ?? activeModel;
                commitModel(slot, v);
              }}
            />
            {#if (providerOf(activeProvider)?.models.length ?? 0) > 0}
              <button
                class="combo-chevron"
                type="button"
                tabindex="-1"
                aria-label="Show {slot.name} models"
                onmousedown={(e) => {
                  e.preventDefault();
                  if (openMenu === slot.id) {
                    openMenu = null;
                  } else {
                    drafts[slot.id] = activeModel;
                    openMenu = slot.id;
                  }
                }}
              >
                <ChevronDown size={14} strokeWidth={1.7} />
              </button>
            {/if}
            {#if isMenuOpen}
              <div class="popover" role="listbox" aria-label="Suggested models">
                {#each suggestions as m (m)}
                  <button
                    class="opt"
                    class:active={m === activeModel}
                    role="option"
                    aria-selected={m === activeModel}
                    onmousedown={(e) => {
                      e.preventDefault();
                      commitModel(slot, m);
                    }}
                  >
                    <span class="opt-name">{m}</span>
                    {#if m === activeModel}
                      <span class="opt-check"><Check size={13} strokeWidth={1.9} /></span>
                    {/if}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      </div>
    {/each}
  </div>
{/if}

<style>
  .slots {
    margin-top: 20px;
    border: 1px solid var(--rule);
    border-radius: 12px;
    background: var(--paper);
  }
  .slot-row {
    padding: 15px 16px;
    border-top: 1px solid var(--rule);
  }
  .slot-row:first-child {
    border-top: 0;
  }

  .slot-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
  }
  .slot-copy {
    min-width: 0;
  }
  h4 {
    margin: 0;
    font-size: 13.5px;
    line-height: 1.3;
    font-weight: 500;
    color: var(--ink);
  }
  .desc {
    margin: 3px 0 0;
    font-size: 13px;
    line-height: 1.4;
    color: var(--faint);
  }

  /* Quiet thinking indicator + solid sliding segmented pill */
  .think {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }
  .think-meter {
    display: inline-flex;
    align-items: flex-end;
    gap: 2px;
    height: 12px;
  }
  .think-meter i {
    display: block;
    width: 2.5px;
    border-radius: 1.5px;
    background: var(--rule);
    transition: background-color 160ms var(--ease-ui), transform 160ms var(--ease-ui);
  }
  .think-meter i.lit {
    background: var(--ink);
  }

  .seg {
    position: relative;
    display: grid;
    grid-template-columns: repeat(5, 44px);
    padding: 2px;
    border-radius: 9px;
    background: var(--shelf);
    border: 1px solid var(--rule);
  }
  .seg-thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 44px;
    height: 24px;
    border-radius: 7px;
    background: var(--paper);
    box-shadow: var(--lift);
    transition: transform 200ms var(--ease-ui);
    pointer-events: none;
  }
  .seg-btn {
    position: relative;
    z-index: 1;
    height: 24px;
    display: grid;
    place-items: center;
    font-size: 11.5px;
    line-height: 1;
    color: var(--faint);
    transition: color 150ms var(--ease-ui);
  }
  .seg-btn:hover {
    color: var(--ink);
  }
  .seg-btn.selected {
    color: var(--ink);
    font-weight: 500;
  }

  /* Aligned 2-column provider + model strip */
  .controls {
    display: grid;
    grid-template-columns: 1fr 1.15fr;
    gap: 10px;
    margin-top: 12px;
  }
  .control {
    width: 100%;
    height: 34px;
    border: 1px solid var(--rule);
    border-radius: 9px;
    background: var(--shelf);
    font: inherit;
    font-size: 13px;
    color: var(--ink);
    outline: none;
    transition: border-color 140ms var(--ease-ui), background-color 140ms var(--ease-ui);
  }
  .control:hover {
    border-color: var(--faint);
  }
  .control:focus {
    background: var(--paper);
    border-color: var(--ink);
  }

  .select-wrap {
    position: relative;
    display: flex;
    align-items: center;
  }
  .dot {
    position: absolute;
    left: 11px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--bulb);
    pointer-events: none;
  }
  .dot.ready {
    background: var(--text-green);
  }
  .select {
    appearance: none;
    padding: 0 30px 0 24px;
    cursor: var(--cursor-pointer);
    text-overflow: ellipsis;
  }
  .chevron {
    position: absolute;
    right: 9px;
    display: grid;
    place-items: center;
    color: var(--faint);
    pointer-events: none;
  }

  .combo {
    position: relative;
    display: flex;
    align-items: center;
  }
  .model-input {
    padding: 0 11px;
  }
  .model-input.has-list {
    padding-right: 30px;
  }
  .combo-chevron {
    position: absolute;
    right: 4px;
    width: 26px;
    height: 26px;
    display: grid;
    place-items: center;
    border-radius: 6px;
    color: var(--faint);
    transition: color 140ms var(--ease-ui);
  }
  .combo-chevron:hover {
    color: var(--ink);
  }

  .popover {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    right: 0;
    z-index: 30;
    max-height: 220px;
    overflow-y: auto;
    padding: 4px;
    border-radius: 10px;
    background: var(--paper);
    box-shadow: var(--lift);
    border: 1px solid var(--rule);
  }
  .opt {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 7px 9px;
    border-radius: 7px;
    text-align: left;
    font-size: 12.5px;
    line-height: 1.3;
    color: var(--graphite);
    transition: background-color 110ms var(--ease-ui), color 110ms var(--ease-ui);
  }
  .opt:hover {
    background: var(--shelf);
    color: var(--ink);
  }
  .opt.active {
    color: var(--ink);
    font-weight: 500;
  }
  .opt-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .opt-check {
    display: grid;
    place-items: center;
    flex: none;
    color: var(--ink);
  }

  @media (prefers-reduced-motion: reduce) {
    .seg-thumb,
    .think-meter i {
      transition: none;
    }
  }
</style>
