<script lang="ts">
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import ChevronRight from 'lucide-svelte/icons/chevron-right';
  import Plus from 'lucide-svelte/icons/plus';
  import Trash2 from 'lucide-svelte/icons/trash-2';
  import { isHttpKind } from '../../api/providers';
  import type { ProviderConfig, ProviderCredentialStatus, ProviderKind } from '../../api/types';
  import {
    credentialStatuses,
    deleteProviderKey,
    setProviderKey,
    settings,
    updateSettings,
  } from '../../state/settings.svelte';

  const providers = $derived(settings.value.providers);
  let openId = $state<string | null>(null);
  let keyDrafts = $state<Record<string, string>>({});
  let savingKey = $state<string | null>(null);

  let adding = $state(false);
  let newName = $state('');
  let newKind = $state<ProviderKind>('openai');
  let newBaseUrl = $state('');
  let newCommand = $state('');
  let newArgs = $state('');
  let newModels = $state('');

  function statusOf(id: string): ProviderCredentialStatus | undefined {
    return credentialStatuses.value.find((c) => c.providerId === id);
  }

  function badge(p: ProviderConfig): { text: string; ok: boolean } {
    if (!isHttpKind(p.kind)) {
      if (p.kind === 'agy-acp' || p.kind === 'acp') return { text: 'ACP', ok: true };
      if (p.kind === 'codex-cli') return { text: 'OAuth CLI', ok: true };
      return { text: 'CLI', ok: true };
    }
    const st = statusOf(p.id);
    if (st?.hasStoredKey) return { text: st.maskedKey ?? 'Key saved', ok: true };
    if (st?.hasEnvKey) return { text: `via $${st.envVar}`, ok: true };
    return { text: 'Needs key', ok: false };
  }

  function updateProvider(id: string, patch: Partial<ProviderConfig>) {
    const next = providers.map((p) => (p.id === id ? { ...p, ...patch } : p));
    void updateSettings({ providers: next });
  }

  async function saveKey(id: string) {
    const raw = (keyDrafts[id] ?? '').trim();
    if (!raw) return;
    savingKey = id;
    try {
      await setProviderKey(id, raw);
      keyDrafts[id] = '';
    } finally {
      savingKey = null;
    }
  }

  async function clearKey(id: string) {
    savingKey = id;
    try {
      await deleteProviderKey(id);
    } finally {
      savingKey = null;
    }
  }

  function removeProvider(id: string) {
    const next = providers.filter((p) => p.id !== id);
    const patch: Partial<typeof settings.value> = { providers: next };
    if (settings.value.readProvider === id) patch.readProvider = 'claude-cli';
    if (settings.value.chatProvider === id) patch.chatProvider = 'claude-cli';
    if (settings.value.drawProvider === id) patch.drawProvider = 'claude-cli';
    if (openId === id) openId = null;
    void updateSettings(patch);
  }

  function addCustom() {
    const name = newName.trim();
    if (!name) return;
    const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'custom';
    const id = providers.some((p) => p.id === slug) ? `${slug}-${Date.now().toString(36)}` : slug;
    const models = newModels
      .split(',')
      .map((m) => m.trim())
      .filter(Boolean);
    const created: ProviderConfig = {
      id,
      name,
      kind: newKind,
      baseUrl: isHttpKind(newKind) ? newBaseUrl.trim() || null : null,
      command: !isHttpKind(newKind) ? newCommand.trim() || null : null,
      args: !isHttpKind(newKind)
        ? newArgs.trim().split(/\s+/).filter(Boolean)
        : [],
      envVar: null,
      models,
      builtin: false,
    };
    void updateSettings({ providers: [...providers, created] });
    openId = id;
    adding = false;
    newName = '';
    newBaseUrl = '';
    newCommand = '';
    newArgs = '';
    newModels = '';
  }
</script>

<div class="group">
  <div class="head">
    <div>
      <h4>Providers &amp; API keys</h4>
      <p class="note tight">Keys stay on this machine in <code>credentials.json</code> (0600) or come from your environment.</p>
    </div>
    <button class="ghost add" onclick={() => (adding = !adding)}>
      <Plus size={13} strokeWidth={1.8} />
      <span>Add custom</span>
    </button>
  </div>

  {#if adding}
    <div class="custom-form">
      <div class="row2">
        <label>
          <span>Name</span>
          <input class="field" placeholder="e.g. Groq, Ollama, Custom ACP" bind:value={newName} />
        </label>
        <label>
          <span>Type</span>
          <select class="field" bind:value={newKind}>
            <option value="openai">OpenAI-compatible API</option>
            <option value="anthropic">Anthropic API</option>
            <option value="gemini">Gemini API</option>
            <option value="acp">ACP Agent (CLI)</option>
          </select>
        </label>
      </div>
      {#if isHttpKind(newKind)}
        <label>
          <span>Base URL</span>
          <input class="field" placeholder="https://api.example.com/v1" bind:value={newBaseUrl} spellcheck="false" />
        </label>
      {:else}
        <div class="row2">
          <label>
            <span>Command</span>
            <input class="field" placeholder="e.g. agy or my-agent" bind:value={newCommand} spellcheck="false" />
          </label>
          <label>
            <span>Arguments</span>
            <input class="field" placeholder="e.g. acp" bind:value={newArgs} spellcheck="false" />
          </label>
        </div>
      {/if}
      <label>
        <span>Suggested models (comma-separated, optional)</span>
        <input class="field" placeholder="model-a, model-b" bind:value={newModels} spellcheck="false" />
      </label>
      <div class="actions">
        <button class="ghost" onclick={() => (adding = false)}>Cancel</button>
        <button class="solid" disabled={!newName.trim()} onclick={addCustom}>Add provider</button>
      </div>
    </div>
  {/if}

  <div class="list">
    {#each providers as p (p.id)}
      {@const b = badge(p)}
      {@const st = statusOf(p.id)}
      {@const open = openId === p.id}
      <div class="prow" class:open>
        <button class="ptoggle" aria-expanded={open} onclick={() => (openId = open ? null : p.id)}>
          <span class="chev">
            {#if open}<ChevronDown size={14} strokeWidth={1.7} />{:else}<ChevronRight size={14} strokeWidth={1.7} />{/if}
          </span>
          <span class="pname">{p.name}</span>
          <span class="status">
            <span class="sdot" class:ok={b.ok} aria-hidden="true"></span>
            <span class="stext" class:ok={b.ok}>{b.text}</span>
          </span>
        </button>

        {#if open}
          <div class="pbody">
            {#if isHttpKind(p.kind)}
              <div class="fieldrow">
                <label class="grow">
                  <span>API key {p.envVar ? `(or $${p.envVar})` : ''}</span>
                  <input
                    class="field"
                    type="password"
                    autocomplete="off"
                    placeholder={st?.hasStoredKey ? `Saved (${st.maskedKey}) — paste to replace` : 'Paste API key'}
                    value={keyDrafts[p.id] ?? ''}
                    oninput={(e) => (keyDrafts[p.id] = e.currentTarget.value)}
                  />
                </label>
                <button
                  class="solid"
                  disabled={savingKey === p.id || !(keyDrafts[p.id] ?? '').trim()}
                  onclick={() => void saveKey(p.id)}
                >Save</button>
                {#if st?.hasStoredKey}
                  <button class="ghost" disabled={savingKey === p.id} onclick={() => void clearKey(p.id)}>Clear</button>
                {/if}
              </div>

              <label>
                <span>Base URL</span>
                <input
                  class="field"
                  value={p.baseUrl ?? ''}
                  spellcheck="false"
                  onchange={(e) => updateProvider(p.id, { baseUrl: e.currentTarget.value.trim() || null })}
                />
              </label>
            {:else}
              <div class="row2">
                <label>
                  <span>Binary / command</span>
                  <input
                    class="field"
                    value={p.command ?? ''}
                    spellcheck="false"
                    onchange={(e) => updateProvider(p.id, { command: e.currentTarget.value.trim() || null })}
                  />
                </label>
                {#if p.kind === 'agy-acp' || p.kind === 'acp' || p.kind === 'codex-cli'}
                  <label>
                    <span>Arguments</span>
                    <input
                      class="field"
                      value={p.args.join(' ')}
                      spellcheck="false"
                      onchange={(e) =>
                        updateProvider(p.id, {
                          args: e.currentTarget.value.trim().split(/\s+/).filter(Boolean),
                        })}
                    />
                  </label>
                {/if}
              </div>
              {#if p.kind === 'codex-cli'}
                <p class="note tight">Uses your OpenAI ChatGPT subscription via <code>codex login</code>.</p>
              {:else if p.kind === 'agy-acp'}
                <p class="note tight">Connects to Antigravity CLI over Agent Client Protocol (<code>agy acp</code>).</p>
              {:else if p.kind === 'claude-cli'}
                <p class="note tight">Uses your Claude subscription via <code>claude</code> CLI.</p>
              {/if}
            {/if}

            {#if !p.builtin}
              <div class="actions">
                <button class="ghost danger" onclick={() => removeProvider(p.id)}>
                  <Trash2 size={13} strokeWidth={1.7} />
                  <span>Remove provider</span>
                </button>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .group { margin-top: 28px; }
  .head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-bottom: 12px; }
  h4 { margin: 0; font-size: 13.5px; line-height: 1.3; font-weight: 500; color: var(--ink); }
  .note { margin: 3px 0 0; font-size: 13px; line-height: 1.4; color: var(--faint); }
  .note code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; color: var(--graphite); }
  .add { flex: none; }

  .list { border: 1px solid var(--rule); border-radius: 12px; overflow: hidden; background: var(--paper); }
  .prow { border-top: 1px solid var(--rule); }
  .prow:first-child { border-top: 0; }
  .prow.open { background: var(--shelf); }
  .ptoggle {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    text-align: left;
    font-size: 13.5px;
    transition: background-color 140ms var(--ease-ui);
  }
  .ptoggle:hover { background: var(--shelf); }
  .chev { display: grid; place-items: center; width: 14px; flex: none; color: var(--faint); }
  .pname { flex: 1; font-weight: 500; color: var(--ink); }

  .status { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; }
  .sdot { width: 6px; height: 6px; border-radius: 50%; background: var(--bulb); flex: none; }
  .sdot.ok { background: var(--text-green); }
  .stext { color: var(--faint); }
  .stext.ok { color: var(--graphite); }

  .pbody { padding: 4px 16px 16px 40px; display: flex; flex-direction: column; gap: 12px; }
  .custom-form {
    margin-bottom: 12px;
    padding: 16px;
    border: 1px solid var(--rule);
    border-radius: 12px;
    background: var(--shelf);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .row2 { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .fieldrow { display: flex; align-items: flex-end; gap: 8px; }
  .grow { flex: 1; }
  label { display: flex; flex-direction: column; gap: 5px; font-size: 12px; color: var(--graphite); }
  .field {
    height: 34px;
    padding: 0 11px;
    border: 1px solid var(--rule);
    border-radius: 9px;
    background: var(--paper);
    font: inherit;
    font-size: 13px;
    color: var(--ink);
    outline: none;
    transition: border-color 140ms var(--ease-ui);
  }
  .field:hover { border-color: var(--faint); }
  .field:focus { border-color: var(--ink); }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 2px; }
  .ghost {
    height: 32px;
    padding: 0 13px;
    border: 1px solid var(--rule);
    border-radius: 999px;
    background: var(--paper);
    font-size: 12.5px;
    color: var(--ink);
    display: inline-flex;
    align-items: center;
    gap: 5px;
    transition: border-color 140ms var(--ease-ui), opacity 140ms var(--ease-ui), transform 140ms var(--ease-step);
  }
  .ghost:hover:not(:disabled) { border-color: var(--faint); }
  .ghost:active:not(:disabled), .solid:active:not(:disabled) { transform: scale(0.97); }
  .ghost:disabled, .solid:disabled { opacity: 0.4; cursor: default; }
  .solid {
    height: 32px;
    padding: 0 14px;
    border-radius: 999px;
    background: var(--ink);
    color: var(--paper);
    font-size: 12.5px;
    font-weight: 500;
    transition: opacity 140ms var(--ease-ui), transform 140ms var(--ease-step);
  }
  .solid:hover:not(:disabled) { opacity: 0.88; }
  .danger { color: var(--text-red); }
  @media (prefers-reduced-motion: reduce) {
    .ghost:active:not(:disabled), .solid:active:not(:disabled) { transform: none; }
  }
</style>
