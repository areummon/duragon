<script lang="ts">
  import { KEYBOARD_ACTIONS, eventBinding, type KeyAction, type KeyScope } from '../../keys';
  import { unfold } from '../../motion/transitions';
  import {
    bindingLabel,
    hasAnyOverride,
    isOverridden,
    resetAllBindings,
    resetBinding,
    setBinding,
  } from '../../state/keymap.svelte';
  import { settings, updateSettings } from '../../state/settings.svelte';
  import SwitchRow from './SwitchRow.svelte';

  const SCOPES: { scope: KeyScope; label: string }[] = [
    { scope: 'navigation', label: 'Navigation' },
    { scope: 'note', label: 'Note' },
  ];

  const VIM_MOTIONS: [string, string][] = [
    ['h j k l', 'Move'],
    ['w b e', 'Word motions'],
    ['0 ^ $', 'Line start / end'],
    ['gg / G', 'Top / bottom'],
    ['i a I A o O', 'Insert mode'],
    ['v', 'Visual selection'],
    ['x dd dw D cc yy p', 'Edit, yank, paste'],
    ['u / Ctrl R', 'Undo / redo'],
  ];

  const VIM_DURAGON: [string, string][] = [
    ['gc', 'Chat'],
    ['gs', 'Settings'],
    ['gw', 'Web'],
    ['gv', 'Voice'],
    ['gm', 'Reading mode'],
    ['za', 'Just my words'],
    ['Esc', 'Normal mode'],
  ];

  const MODIFIER_KEYS = new Set(['Control', 'Meta', 'Alt', 'Shift']);

  let rebinding = $state<KeyAction | null>(null);
  let errorRow = $state<KeyAction | null>(null);
  let errorMessage = $state<string | null>(null);

  function beginRebind(id: KeyAction) {
    rebinding = id;
    errorRow = null;
    errorMessage = null;
  }

  function cancelRebind() {
    rebinding = null;
    errorRow = null;
    errorMessage = null;
  }

  function resetOne(id: KeyAction) {
    if (errorRow === id) {
      errorRow = null;
      errorMessage = null;
    }
    resetBinding(id);
  }

  function resetAll() {
    cancelRebind();
    resetAllBindings();
  }

  $effect(() => {
    const target = rebinding;
    if (!target) return;

    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      if (e.key === 'Escape') {
        cancelRebind();
        return;
      }
      if (MODIFIER_KEYS.has(e.key)) return;

      const binding = eventBinding(e);
      if (!binding) return;

      const res = setBinding(target, binding);
      if (!res.ok) {
        errorRow = target;
        errorMessage = res.error;
        return;
      }

      rebinding = null;
      errorRow = null;
      errorMessage = null;
    };

    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  });
</script>

<section>
  <h3>Editing</h3>
  <div class="group">
    <SwitchRow
      title="Vim keybindings"
      desc="Modal editing in the note (Normal, Insert, Visual) with quick shortcuts for chat, web, voice, and reading mode."
      on={settings.value.vimMode}
      onchange={(v) => void updateSettings({ vimMode: v })}
    />

    {#if settings.value.vimMode}
      <div class="vim-ref" transition:unfold={{ duration: 200 }}>
        <div class="vim-ref-inner">
          <div class="vim-col">
            <div class="vim-head">Motions &amp; edits</div>
            <dl class="vim-dl">
              {#each VIM_MOTIONS as [keys, label] (keys)}
                <div class="vim-item">
                  <dt><kbd class="vim-key">{keys}</kbd></dt>
                  <dd>{label}</dd>
                </div>
              {/each}
            </dl>
          </div>
          <div class="vim-col">
            <div class="vim-head">Duragon in Normal mode</div>
            <dl class="vim-dl">
              {#each VIM_DURAGON as [keys, label] (keys)}
                <div class="vim-item">
                  <dt><kbd class="vim-key">{keys}</kbd></dt>
                  <dd>{label}</dd>
                </div>
              {/each}
            </dl>
          </div>
        </div>
      </div>
    {/if}
  </div>
</section>

<section>
  <h3>Shortcuts</h3>
  <p class="sub">Click Rebind, then press a new shortcut. Escape cancels.</p>

  {#each SCOPES as { scope, label } (scope)}
    {@const actions = KEYBOARD_ACTIONS.filter((a) => a.scope === scope)}
    <div class="group">
      <h4>{label}</h4>
      <div class="list" data-testid="kbd-scope-{scope}">
        {#each actions as def (def.id)}
          {@const capturing = rebinding === def.id}
          {@const overridden = isOverridden(def.id)}
          <div class="row-wrap" data-testid="kbd-row-{def.id}">
            <div class="row">
              <span class="label">{def.label}</span>
              <div class="controls">
                <kbd class="pill" class:capturing>
                  {capturing ? 'Press keys…' : bindingLabel(def.id)}
                </kbd>
                {#if capturing}
                  <button type="button" class="action" onclick={cancelRebind}>Cancel</button>
                {:else}
                  <button
                    type="button"
                    class="action"
                    data-testid="kbd-rebind-{def.id}"
                    onclick={() => beginRebind(def.id)}
                  >
                    Rebind
                  </button>
                  {#if overridden}
                    <button
                      type="button"
                      class="action muted"
                      data-testid="kbd-reset-{def.id}"
                      onclick={() => resetOne(def.id)}
                    >
                      Reset
                    </button>
                  {/if}
                {/if}
              </div>
            </div>
            {#if errorRow === def.id && errorMessage}
              <p class="row-error" role="alert">{errorMessage}</p>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/each}

  <div class="footer">
    <span class="footnote">Stored on this device. Overrides only.</span>
    <button
      type="button"
      class="reset-all"
      data-testid="kbd-reset-all"
      disabled={!hasAnyOverride()}
      onclick={resetAll}
    >
      Reset all
    </button>
  </div>
</section>

<style>
  section { margin-top: 34px; }
  section:first-child { margin-top: 0; }
  h3 { margin: 0; font-size: 16px; line-height: 1.3; font-weight: 500; }
  .sub { margin-top: 3px; font-size: 13.5px; line-height: 1.45; color: var(--faint); }
  .group { margin-top: 18px; }
  h4 { margin: 0 0 10px; font-size: 13.5px; line-height: 1.3; font-weight: 500; color: var(--ink); }

  .vim-ref {
    margin-top: 10px;
  }
  .vim-ref-inner {
    display: grid;
    grid-template-columns: 1.25fr 1fr;
    gap: 20px;
    padding: 14px 16px;
    border: 1px solid var(--rule);
    border-radius: 12px;
    background: var(--shelf);
  }
  .vim-head {
    font-size: 12.5px;
    line-height: 1.3;
    font-weight: 500;
    color: var(--faint);
    margin-bottom: 10px;
  }
  .vim-dl {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .vim-item {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    font-size: 12.5px;
    line-height: 1.35;
  }
  .vim-item dt {
    flex: none;
  }
  .vim-item dd {
    margin: 0;
    color: var(--graphite);
    text-align: right;
  }
  .vim-key {
    display: inline-block;
    font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
    font-size: 12px;
    color: var(--ink);
    text-box: trim-both cap alphabetic;
  }

  .list {
    border: 1px solid var(--rule);
    border-radius: 12px;
  }
  .row-wrap {
    border-top: 1px solid var(--rule);
  }
  .row-wrap:first-child {
    border-top: 0;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 44px;
    padding: 8px 16px;
  }
  .label {
    font-size: 13.5px;
    line-height: 1.3;
    font-weight: 500;
    color: var(--ink);
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: none;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 3px 8px;
    border: 1px solid var(--rule);
    border-radius: 6px;
    background: var(--shelf);
    font-family: var(--font-sans);
    font-size: 12.5px;
    line-height: 1.3;
    color: var(--ink);
    text-box: trim-both cap alphabetic;
    transition: border-color var(--dur-ui) var(--ease-ui), color var(--dur-ui) var(--ease-ui);
  }
  .pill.capturing {
    border-color: var(--ink);
    color: var(--ink);
  }
  .action {
    font-size: 12.5px;
    line-height: 1.3;
    color: var(--graphite);
    padding: 4px 2px;
    transition: color var(--dur-ui) var(--ease-ui);
  }
  .action:hover {
    color: var(--ink);
  }
  .action.muted {
    color: var(--faint);
  }
  .action.muted:hover {
    color: var(--ink);
  }
  .row-error {
    margin: 0;
    padding: 0 16px 10px;
    font-size: 12.5px;
    line-height: 1.35;
    color: var(--text-red);
  }

  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-top: 14px;
  }
  .footnote {
    font-size: 12.5px;
    line-height: 1.35;
    color: var(--faint);
  }
  .reset-all {
    flex: none;
    height: 30px;
    padding: 0 14px;
    border: 1px solid var(--rule);
    border-radius: 999px;
    background: transparent;
    font-size: 13px;
    line-height: 1;
    color: var(--graphite);
    text-box: trim-both cap alphabetic;
    transition: color var(--dur-ui) var(--ease-ui), border-color var(--dur-ui) var(--ease-ui), background-color var(--dur-ui) var(--ease-ui);
  }
  .reset-all:hover:not(:disabled) {
    color: var(--ink);
    border-color: var(--riddle-line);
    background: var(--riddle-wash);
  }
  .reset-all:disabled {
    opacity: 0.4;
    cursor: default;
  }

  @media (max-width: 640px) {
    .vim-ref-inner {
      grid-template-columns: 1fr;
    }
  }
</style>
