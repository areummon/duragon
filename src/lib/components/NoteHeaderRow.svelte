<script lang="ts">
  import { Check, ChevronDown, Globe, Lightbulb, MessageCircle, Pencil, Plus, Share, Volume2 } from 'lucide-svelte';
  import type { ReadingMode } from '../api/types';
  import { app, exportNote, exportPdf, newMindForNote, openCustomize, updateMeta } from '../state/app.svelte';
  import { notes } from '../state/notes.svelte';
  import { riddlerName } from '../state/riddler';
  import { layout } from '../state/layout.svelte';
  import Blot from './Blot.svelte';
  import Popover from './Popover.svelte';
  import Toggle from './Toggle.svelte';

  type Menu = 'riddler' | 'mode' | 'export' | null;
  let menu = $state<Menu>(null);
  const toggle = (m: Exclude<Menu, null>) => () => (menu = menu === m ? null : m);
  const close = () => (menu = null);

  const note = $derived(app.note!);
  const riddler = $derived(notes.riddlers.find((r) => r.id === note.riddlerId));

  const MODES: { id: ReadingMode; label: string; description: string }[] = [
    { id: 'thinks', label: 'Thinks with you', description: 'Reads along and speaks up when something’s worth a second look.' },
    { id: 'asks', label: 'When you ask', description: 'Quiet until you ask: say its name, select words, or open the chat.' },
  ];
  const mode = $derived(MODES.find((m) => m.id === note.mode) ?? MODES[0]);

  function pickRiddler(id: string) {
    close();
    void updateMeta({ riddlerId: id });
  }
  function pickMode(id: ReadingMode) {
    close();
    void updateMeta({ mode: id });
  }
</script>

<div class="row" class:pill={layout.scrolled} role="toolbar" aria-label="Note settings">
  <div class="slot">
    <button class="item" aria-haspopup="menu" aria-expanded={menu === 'riddler'} onclick={toggle('riddler')}>
      <Blot size={15} /><span>{riddler?.name ?? 'Duragon'}</span><ChevronDown class="chev" size={13} strokeWidth={1.6} />
    </button>
    {#snippet mindOption(r: (typeof notes.riddlers)[number])}
      <button class="menu-item opt" role="menuitemradio" aria-checked={r.id === note.riddlerId} onclick={() => pickRiddler(r.id)}>
        <Blot size={15} />
        <span class="text"><span class="name">{r.name}</span><span class="desc">{r.description}</span></span>
        {#if r.id === note.riddlerId}<Check size={15} strokeWidth={1.6} />{/if}
      </button>
    {/snippet}
    <Popover open={menu === 'riddler'} onclose={close} width={290} label="Mind">
      {#each notes.riddlers.filter((r) => r.builtin) as r (r.id)}{@render mindOption(r)}{/each}
      {#if notes.riddlers.some((r) => !r.builtin)}
        <p class="group-label">Yours</p>
        {#each notes.riddlers.filter((r) => !r.builtin) as r (r.id)}{@render mindOption(r)}{/each}
      {/if}
      <div class="foot">
        <button class="foot-btn" role="menuitem" onclick={() => { close(); newMindForNote(); }}><Plus size={14} strokeWidth={1.5} />New Mind</button>
        <button class="foot-btn manage" role="menuitem" onclick={() => { close(); openCustomize('minds'); }}>Manage</button>
      </div>
    </Popover>
  </div>

  {#if !note.brief}
    <button class="item" aria-expanded={app.tellOpen} onclick={() => (app.tellOpen = !app.tellOpen)}>
      <Pencil size={15} strokeWidth={1.5} /><span>Tell {riddlerName()}</span>
    </button>
  {/if}

  <div class="group">
    <div class="slot">
      <button class="item" aria-haspopup="menu" aria-expanded={menu === 'mode'} onclick={toggle('mode')}>
        {#if note.mode === 'thinks'}<Lightbulb size={15} strokeWidth={1.5} style="color: var(--bulb)" />{:else}<MessageCircle size={15} strokeWidth={1.5} />{/if}<span>{mode.label}</span><ChevronDown class="chev" size={13} strokeWidth={1.6} />
      </button>
      <Popover open={menu === 'mode'} onclose={close} width={300} label="Reading mode">
        {#each MODES as m (m.id)}
          <button class="menu-item opt" role="menuitemradio" aria-checked={m.id === note.mode} onclick={() => pickMode(m.id)}>
            {#if m.id === 'thinks'}<Lightbulb size={15} strokeWidth={1.5} style={m.id === note.mode ? 'color: var(--bulb)' : ''} />{:else}<MessageCircle size={15} strokeWidth={1.5} />{/if}
            <span class="text"><span class="name">{m.label}</span><span class="desc">{m.description}</span></span>
            {#if m.id === note.mode}<Check size={15} strokeWidth={1.6} />{/if}
          </button>
        {/each}
      </Popover>
    </div>

    <button class="item" role="switch" aria-checked={note.voiceOn} onclick={() => updateMeta({ voiceOn: !note.voiceOn })}>
      <Volume2 size={15} strokeWidth={1.5} /><span>Voice</span><Toggle on={note.voiceOn} />
    </button>
    <button class="item" role="switch" aria-checked={note.webOn} onclick={() => updateMeta({ webOn: !note.webOn })}>
      <Globe size={15} strokeWidth={1.5} /><span>Web</span><Toggle on={note.webOn} />
    </button>
  </div>

  <div class="slot end">
    <button class="item" aria-haspopup="menu" aria-expanded={menu === 'export'} onclick={toggle('export')}>
      <Share size={15} strokeWidth={1.5} /><span>Export</span>
    </button>
    <Popover open={menu === 'export'} onclose={close} align="right" width={150} label="Export">
      <button class="menu-item" role="menuitem" onclick={() => { close(); void exportNote('md'); }}>Markdown</button>
      <button class="menu-item" role="menuitem" onclick={() => { close(); void exportNote('docx'); }}>Word</button>
      <button class="menu-item" role="menuitem" onclick={() => { close(); exportPdf(); }}>PDF</button>
    </Popover>
  </div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    column-gap: 4px;
    row-gap: 2px;
    min-height: 30px;
    margin-left: -8px;
    margin-right: -8px;
    font-size: 14px;
    line-height: 1;
    color: var(--graphite);
    letter-spacing: 0;
  }
  .row {
    position: sticky;
    top: 14px;
    z-index: 15;
    border: 1px solid transparent;
    border-radius: 999px;
    background: transparent;
    transition: border-color 200ms var(--ease-ui), background-color 200ms var(--ease-ui), padding 200ms var(--ease-ui);
  }
  .row.pill { border-color: var(--rule); background: var(--paper); padding: 0 10px; margin-left: -18px; margin-right: -18px; }
  .row.pill { border-radius: 18px; }
  .slot { position: relative; }
  .group { display: flex; align-items: center; gap: 2px; }
  .slot.end { margin-left: auto; }
  .item {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 30px;
    padding: 0 8px;
    border-radius: 6px;
    white-space: nowrap;
    transition: color var(--dur-ui) var(--ease-ui), background-color var(--dur-ui) var(--ease-ui);
  }
  .item:hover, .item[aria-expanded='true'] { color: var(--ink); background: var(--shelf); }
  .item :global(svg) { flex: none; }
  .item :global(.chev) { margin-left: -1px; color: var(--faint); }
  .item[role='switch'] { gap: 7px; }
  .item[role='switch'] :global(.track) { margin-left: 2px; }

  .opt { align-items: flex-start; padding: 8px 10px; gap: 12px; }
  .opt > :global(svg:first-child), .opt > :global(.blot) { margin-top: 2px; }
  .text { display: flex; flex-direction: column; gap: 3px; flex: 1; min-width: 0; }
  .name { font-size: 14px; }
  .desc { font-size: 12.5px; line-height: 1.4; color: var(--faint); }
  .opt > :global(svg:last-child) { margin-top: 2px; color: var(--graphite); }
  .group-label { padding: 8px 10px 4px; font-size: 12px; line-height: 1.3; color: var(--faint); }
  .foot { display: flex; align-items: center; gap: 4px; margin-top: 4px; padding: 6px 4px 0; border-top: 1px solid var(--rule); }
  .foot-btn {
    display: inline-flex; align-items: center; gap: 6px; height: 32px; padding: 0 8px;
    border-radius: 8px; font-size: 13px; color: var(--graphite);
    transition: background-color 100ms var(--ease-ui), color 100ms var(--ease-ui);
  }
  .foot-btn.manage { margin-left: auto; color: var(--faint); }
  .foot-btn:hover, .foot-btn:focus-visible { background: var(--shelf); color: var(--ink); outline: none; }
</style>
