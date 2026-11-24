<script lang="ts">
  import { kbd } from '../../keys';

  const KEYS: [string, string][] = [
    [kbd('K'), 'Find a note, or name a new one'],
    [kbd('N'), 'New note'],
    ['/', 'Commands: headings, lists, quote, code block, divider'],
    ['---', 'A divider'],
    ['```', 'A code block. ``` and Enter on its last line closes it'],
    ['Esc', 'Close the menu, panel or dialog on top'],
    ['Tab', 'Move to the next control, or indent in a list or code'],
    ['Shift Tab', 'Move back, or outdent'],
    [kbd('J'), 'Open the chat'],
    [kbd(','), 'Customize'],
    [kbd('C', true), 'Copy this note'],
    [kbd('E', true), 'Export this note'],
    [kbd('.', true), 'Just my words'],
    ['Alt', 'Hold to talk. Tap twice for hands-free'],
    [kbd('↵'), 'With words selected: ask Duragon about them'],
  ];
</script>

<p class="intro">Write in the note. Duragon reads along and asks in the margin. Start a line with its name and it answers right under it. Select words to ask about them, or open the chat to talk about the whole note.</p>

<h4>Shortcuts</h4>
<dl>
  {#each KEYS as [k, what] (k)}<div><dt>{k}</dt><dd>{what}</dd></div>{/each}
</dl>

<h4>Your notes</h4>
<p class="intro">Everything stays on this computer, in <code>~/.local/share/dev.duragon.app/duragon.db</code>. Duragon talks to Claude through your installed <code>claude</code> command, using your own subscription. Voice runs fully offline once its models are installed.</p>

<style>
  .intro { max-width: 480px; font-size: 14px; line-height: 1.55; color: var(--graphite); }
  h4 { margin: 28px 0 10px; font-size: 13px; font-weight: 400; color: var(--faint); }
  dl { margin: 0; }
  dl > div { display: flex; gap: 16px; padding: 9px 0; border-top: 1px solid var(--rule); font-size: 13.5px; }
  dl > div:first-child { border-top: 0; }
  dt { width: 110px; flex: none; color: var(--ink); }
  dd { margin: 0; color: var(--graphite); }
  code { font-size: 12.5px; padding: 1px 5px; border-radius: 4px; background: var(--code); }
</style>
