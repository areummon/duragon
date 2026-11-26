<script lang="ts">
  import Copy from 'lucide-svelte/icons/copy';
  import Download from 'lucide-svelte/icons/download';
  import EyeOff from 'lucide-svelte/icons/eye-off';
  import FolderOutput from 'lucide-svelte/icons/folder-output';
  import KeyRound from 'lucide-svelte/icons/key-round';
  import Sliders from 'lucide-svelte/icons/sliders';
  import Upload from 'lucide-svelte/icons/upload';
  import Users from 'lucide-svelte/icons/users';
  import { copyNote, exportAllMarkdown, exportNote, importMarkdownNote, openCustomize, toggleJustMyWords } from '../state/app.svelte';
  import { bindingLabel } from '../state/keymap.svelte';
  import { settings } from '../state/settings.svelte';

  let { onpick }: { onpick: () => void } = $props();
  const run = (fn: () => void) => () => {
    onpick();
    fn();
  };
</script>

<button class="menu-item" role="menuitem" onclick={run(() => void copyNote())}>
  <Copy size={15} strokeWidth={1.5} />Copy this note<span class="kbd">{bindingLabel('copy-note')}</span>
</button>
<button class="menu-item" role="menuitem" onclick={run(() => void exportNote('md'))}>
  <Download size={15} strokeWidth={1.5} />Export this note<span class="kbd">{bindingLabel('export-note')}</span>
</button>
<button class="menu-item" role="menuitem" onclick={run(() => void importMarkdownNote())}>
  <Upload size={15} strokeWidth={1.5} />Import Markdown…
</button>
<button class="menu-item" role="menuitem" onclick={run(() => void exportAllMarkdown())}>
  <FolderOutput size={15} strokeWidth={1.5} />Export all notes…
</button>
<button class="menu-item" role="menuitemcheckbox" aria-checked={settings.value.justMyWords} onclick={run(() => void toggleJustMyWords())}>
  <EyeOff size={15} strokeWidth={1.5} />Just my words<span class="kbd">{bindingLabel('just-my-words')}</span>
</button>
<div class="menu-sep"></div>
<button class="menu-item" role="menuitem" onclick={run(() => openCustomize())}>
  <Sliders size={15} strokeWidth={1.5} />Customize<span class="kbd">{bindingLabel('open-customize')}</span>
</button>
<button class="menu-item" role="menuitem" onclick={run(() => openCustomize('providers'))}>
  <KeyRound size={15} strokeWidth={1.5} />Providers
</button>
<button class="menu-item" role="menuitem" onclick={run(() => openCustomize('minds'))}>
  <Users size={15} strokeWidth={1.5} />Minds
</button>
