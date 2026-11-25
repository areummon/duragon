<script lang="ts">
  import X from 'lucide-svelte/icons/x';
  import { tick } from 'svelte';
  import { BUILTIN_RIDDLER_ID, type Frequency, type Tone } from '../../api/types';
  import { RIDDLER_PRESETS } from '../../api/presets';
  import { closeOnEscape } from '../../escape';
  import { pop, veil } from '../../motion/transitions';
  import { app, updateMeta } from '../../state/app.svelte';
  import { notes, removeMind, saveMind } from '../../state/notes.svelte';
  import { showToast } from '../../state/toast.svelte';
  import Blot from '../Blot.svelte';

  const TONES: [Tone, string][] = [['plain', 'Plain'], ['warm', 'Warm'], ['blunt', 'Blunt'], ['riddle', 'Riddle']];
  const FREQS: [Frequency, string][] = [['rarely', 'Rarely'], ['matters', 'When it matters'], ['often', 'Often']];

  const editing = $derived(app.mindEditor?.id ? notes.riddlers.find((r) => r.id === app.mindEditor?.id) : undefined);

  let name = $state('');
  let description = $state('');
  let howItThinks = $state('');
  let tone = $state<Tone>('plain');
  let frequency = $state<Frequency>('matters');
  let preset = $state<string | null>(null);
  let nameInput = $state<HTMLInputElement>();
  let busy = $state(false);
  let loadedFor: string | null | undefined;

  $effect(() => {
    const ed = app.mindEditor;
    if (!ed) {
      loadedFor = undefined;
      return;
    }
    if (loadedFor === ed.id) return;
    loadedFor = ed.id;
    const r = ed.id ? notes.riddlers.find((x) => x.id === ed.id) : undefined;
    name = r?.name ?? '';
    description = r?.description ?? '';
    howItThinks = r?.howItThinks ?? '';
    tone = r?.tone ?? 'plain';
    frequency = r?.frequency ?? 'matters';
    preset = null;
    void tick().then(() => nameInput?.focus());
  });

  function fromPreset(p: (typeof RIDDLER_PRESETS)[number] | null) {
    preset = p?.name ?? 'Blank';
    name = p?.name ?? '';
    description = p?.description ?? '';
    howItThinks = p?.howItThinks ?? '';
    tone = p?.tone ?? 'plain';
    frequency = p?.frequency ?? 'matters';
  }

  const valid = $derived(name.trim() !== '' && howItThinks.trim() !== '');
  const close = () => (app.mindEditor = null);

  async function save() {
    if (!valid || busy) return;
    busy = true;
    const assign = app.mindEditor?.id === null && app.mindEditor.assign;
    try {
      const saved = await saveMind({
        id: editing?.id ?? crypto.randomUUID(),
        name: name.trim(),
        description: description.trim(),
        howItThinks: howItThinks.trim(),
        tone,
        frequency,
        builtin: false,
      });
      close();
      if (assign) await updateMeta({ riddlerId: saved.id });
      showToast(`${saved.name} is ready.`);
    } finally {
      busy = false;
    }
  }

  async function remove() {
    if (!editing) return;
    const id = editing.id;
    close();
    await removeMind(id);
    if (app.note?.riddlerId === id) await updateMeta({ riddlerId: BUILTIN_RIDDLER_ID });
  }

  $effect(() => {
    if (app.mindEditor) return closeOnEscape(close);
  });

  function onkeydown(e: KeyboardEvent) {
    if (!app.mindEditor) return;
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      void save();
    }
  }
  const ph = RIDDLER_PRESETS[0];
</script>

<svelte:window onkeydowncapture={onkeydown} />

{#if app.mindEditor}
  <div class="scrim" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && close()} transition:veil|global>
    <div class="card" role="dialog" aria-modal="true" aria-label={editing ? `Edit ${editing.name}` : 'New Mind'} in:pop|global out:pop={{ duration: 140 }}>
      <button class="x" aria-label="Close" onclick={close}><X size={17} strokeWidth={1.5} /></button>
      <h2><Blot size={14} seed="duragon" />{editing ? `Edit ${editing.name}` : 'New Mind'}</h2>
      <p class="intro">A Mind reads your notes: what it looks for, and how it asks. Give any note the one it needs.</p>

      {#if !editing}
        <div class="field">
          <div class="cap">Start from</div>
          <div class="chips">
            {#each RIDDLER_PRESETS as p (p.name)}
              <button class="chip" class:on={preset === p.name} aria-pressed={preset === p.name} onclick={() => fromPreset(p)}>{p.name}</button>
            {/each}
            <button class="chip" class:on={preset === 'Blank'} aria-pressed={preset === 'Blank'} onclick={() => fromPreset(null)}>Blank</button>
          </div>
        </div>
      {/if}

      <label class="field">
        <span class="lab">Name</span>
        <input bind:this={nameInput} bind:value={name} placeholder={ph.name} maxlength="40" />
      </label>
      <label class="field">
        <span class="lab">Description <i>Shown when you pick it</i></span>
        <input bind:value={description} placeholder={ph.description} maxlength="120" />
      </label>
      <label class="field">
        <span class="lab">How it thinks <i>What it looks for, pushes on, and leaves alone</i></span>
        <textarea bind:value={howItThinks} rows="5" placeholder="Read like a senior engineer reviewing a design doc. Ask about scale and failure. Ignore style."></textarea>
      </label>

      <div class="field">
        <div class="lab">Voice</div>
        <div class="chips" role="radiogroup" aria-label="Voice">
          {#each TONES as [id, label] (id)}
            <button class="chip" class:on={tone === id} role="radio" aria-checked={tone === id} onclick={() => (tone = id)}>{label}</button>
          {/each}
        </div>
      </div>
      <div class="field">
        <div class="lab">How often it speaks up</div>
        <div class="chips" role="radiogroup" aria-label="How often it speaks up">
          {#each FREQS as [id, label] (id)}
            <button class="chip" class:on={frequency === id} role="radio" aria-checked={frequency === id} onclick={() => (frequency = id)}>{label}</button>
          {/each}
        </div>
      </div>

      <div class="actions">
        {#if editing}<button class="quiet danger" onclick={remove}>Delete</button>{/if}
        <span class="grow"></span>
        <button class="quiet" onclick={close}>Cancel</button>
        <button class="primary" disabled={!valid || busy} onclick={save}>{editing ? 'Save' : 'Create Mind'}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .scrim { position: fixed; inset: 0; z-index: 75; display: grid; place-items: center; padding: 24px; background: var(--veil-soft); overflow-y: auto; }
  .card {
    position: relative;
    width: min(622px, 100%);
    padding: 24px 32px 24px;
    border-radius: 14px;
    background: var(--paper);
    box-shadow: var(--lift);
    font-size: 14px;
    line-height: 1.4;
  }
  .x { position: absolute; top: 16px; right: 16px; display: grid; place-items: center; width: 28px; height: 28px; color: var(--graphite); }
  .x:hover { color: var(--ink); }
  h2 { display: flex; align-items: center; gap: 8px; margin: 0; font-size: 19px; line-height: 1.3; font-weight: 500; }
  .intro { margin-top: 6px; max-width: 500px; font-size: 13.5px; color: var(--graphite); }
  .field { display: block; margin-top: 20px; }
  .cap { margin-bottom: 10px; font-size: 13px; color: var(--faint); }
  .lab { display: block; margin-bottom: 8px; font-size: 13.5px; font-weight: 500; }
  .lab i { margin-left: 6px; font-style: normal; font-weight: 400; font-size: 12.5px; color: var(--faint); }
  input, textarea {
    width: 100%;
    padding: 0 14px;
    border: 1px solid var(--rule);
    border-radius: 10px;
    background: transparent;
    font: inherit;
    font-size: 14.5px;
    color: var(--ink);
    outline: none;
    transition: border-color 150ms var(--ease-ui);
  }
  input { height: 42px; }
  textarea { padding: 10px 14px; line-height: 1.5; resize: vertical; min-height: 120px; }
  input:focus, textarea:focus { border-color: var(--faint); }
  ::placeholder { color: var(--faint); opacity: 0.8; }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { height: 32px; padding: 0 12px; border: 1px solid var(--rule); border-radius: 999px; font-size: 13.5px; color: var(--ink); transition: background-color 150ms var(--ease-ui), color 150ms var(--ease-ui), border-color 150ms var(--ease-ui); }
  @media (hover: hover) and (pointer: fine) { .chip:not(.on):hover { border-color: var(--faint); } }
  .chip.on { background: var(--ink); border-color: var(--ink); color: var(--paper); }
  .actions { display: flex; align-items: center; gap: 6px; margin-top: 26px; }
  .grow { flex: 1; }
  .quiet { height: 36px; padding: 0 14px; font-size: 14px; color: var(--graphite); transition: color 150ms var(--ease-ui); }
  .quiet:hover { color: var(--ink); }
  .quiet.danger { color: var(--faint); }
  .quiet.danger:hover { color: var(--text-red); }
  .primary { height: 36px; padding: 0 18px; border-radius: 999px; background: var(--ink); color: var(--paper); font-size: 14px; transition: opacity 150ms var(--ease-ui), transform 160ms var(--ease-step); }
  .primary:disabled { opacity: 0.35; cursor: default; }
  /* Gives a little under the press. */
  .primary:active:not(:disabled) { transform: scale(0.97); }
  @media (prefers-reduced-motion: reduce) { .primary:active:not(:disabled) { transform: none; } }
</style>
