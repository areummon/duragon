<script lang="ts">
  import { settings, updateSettings } from '../../state/settings.svelte';
  import Switch from './Switch.svelte';

  const LIMIT = 700;
  /** The writer's own words, until they are stored. */
  let edit = $state<string | null>(null);
  const text = $derived(edit ?? settings.value.aboutYou);
  let timer = 0;

  function input(e: Event) {
    edit = (e.currentTarget as HTMLTextAreaElement).value.slice(0, LIMIT);
    window.clearTimeout(timer);
    timer = window.setTimeout(() => void save(), 300);
  }
  async function save() {
    window.clearTimeout(timer);
    const mine = edit;
    if (mine === null) return;
    if ((await updateSettings({ aboutYou: mine })) && edit === mine) edit = null;
  }
</script>

<p class="intro">A short paragraph Duragon keeps about you, so every note starts out knowing it. It adds to it when you tell it something about yourself. Only you can see it.</p>

<div class="learn">
  <div>
    <div class="t">Duragon learns about you</div>
    <div class="d">When you say something lasting about yourself, like what you study or how you like feedback, it works it in here.</div>
  </div>
  <Switch on={settings.value.learnAboutYou} label="Duragon learns about you" onchange={(v) => void updateSettings({ learnAboutYou: v })} />
</div>

<textarea
  value={text}
  oninput={input}
  onblur={save}
  maxlength={LIMIT}
  rows="5"
  aria-label="About you"
  placeholder="Nothing yet. Write what Duragon should know: what you do, what you’re working toward, how you like feedback."
></textarea>
<div class="count">{text.length} of {LIMIT}</div>

<style>
  .intro { max-width: 460px; font-size: 14px; line-height: 1.5; color: var(--graphite); }
  .learn { display: flex; align-items: center; gap: 24px; margin-top: 22px; padding: 14px 16px; border: 1px solid var(--rule); border-radius: 12px; }
  .learn > div:first-child { flex: 1; }
  .t { font-size: 13.5px; font-weight: 500; }
  .d { margin-top: 3px; font-size: 13px; line-height: 1.4; color: var(--faint); }
  textarea {
    display: block;
    width: 100%;
    min-height: 140px;
    margin-top: 12px;
    padding: 14px 16px;
    border: 1px solid var(--rule);
    border-radius: 12px;
    background: transparent;
    font: inherit;
    font-size: 14.5px;
    line-height: 1.6;
    color: var(--ink);
    outline: none;
    resize: vertical;
    transition: border-color 150ms var(--ease-ui);
  }
  textarea:focus { border-color: var(--faint); }
  textarea::placeholder { color: var(--faint); opacity: 0.8; }
  .count { margin-top: 12px; font-size: 12.5px; color: var(--faint); font-variant-numeric: tabular-nums; }
</style>
