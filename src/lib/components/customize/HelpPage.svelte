<script lang="ts">
  import { bindingLabel } from '../../state/keymap.svelte';

  const KEYS = $derived<[string, string][]>([
    [bindingLabel('find-note'), 'Find a note, or name a new one'],
    [bindingLabel('new-note'), 'New note'],
    ['/', 'Commands: headings, lists, quote, code block, divider'],
    ['---', 'A divider'],
    ['```', 'A code block. ``` and Enter on its last line closes it'],
    ['Esc', 'Close the menu, panel or dialog on top'],
    ['Tab', 'Move to the next control, or indent in a list or code'],
    ['Shift Tab', 'Move back, or outdent'],
    [bindingLabel('toggle-chat'), 'Open the chat'],
    [bindingLabel('open-customize'), 'Customize'],
    [bindingLabel('copy-note'), 'Copy this note'],
    [bindingLabel('export-note'), 'Export this note'],
    [bindingLabel('just-my-words'), 'Just my words'],
    ['Alt', 'Hold to talk. Tap twice for hands-free'],
    [bindingLabel('riddle-selection'), 'With words selected: ask Duragon about them'],
  ]);
</script>

<p class="intro">Write in the note. Duragon reads along and asks in the margin. Start a line with its name and it answers right under it. Select words to ask about them, or open the chat to talk about the whole note.</p>

<h4>Shortcuts</h4>
<dl>
  {#each KEYS as [k, what] (what)}<div><dt>{k}</dt><dd>{what}</dd></div>{/each}
</dl>

<h4>Your notes</h4>
<p class="intro">Your notes are kept on this computer, in <code>~/.local/share/dev.duragon.app/duragon.db</code>. Duragon’s trash is the folder <code>.trash</code> beside it. What Duragon reads goes to the model you picked for the job. That is Claude, through your installed <code>claude</code> command, unless you chose another in Customize.</p>
<p class="intro more">Duragon has no telemetry, and asks the tools it runs to turn theirs off. Email addresses and common kinds of API key in what it sends are masked. A key goes only to its own provider, to sign you in. A tool that uses your own account, such as <code>claude</code>, still tells its maker whose account it is, and Claude Code also tells the model that account’s email. A model on your own computer tells no one. Voice runs fully offline once its models are installed.</p>

<style>
  .intro { max-width: 480px; font-size: 14px; line-height: 1.55; color: var(--graphite); }
  .more { margin-top: 10px; }
  h4 { margin: 28px 0 10px; font-size: 13px; font-weight: 400; color: var(--faint); }
  dl { margin: 0; }
  dl > div { display: flex; gap: 16px; padding: 9px 0; border-top: 1px solid var(--rule); font-size: 13.5px; }
  dl > div:first-child { border-top: 0; }
  dt { width: 110px; flex: none; color: var(--ink); }
  dd { margin: 0; color: var(--graphite); }
  code { font-size: 12.5px; padding: 1px 5px; border-radius: 4px; background: var(--code); }
</style>
