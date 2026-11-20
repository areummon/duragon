<script lang="ts">
  import Copy from 'lucide-svelte/icons/copy';
  import Download from 'lucide-svelte/icons/download';
  import EyeOff from 'lucide-svelte/icons/eye-off';
  import Sliders from 'lucide-svelte/icons/sliders';
  import Users from 'lucide-svelte/icons/users';
  import { copyNote, openCustomize, exportNote, toggleJustMyWords } from '../state/app.svelte';
  import { settings } from '../state/settings.svelte';
  import { kbd } from '../keys';

  let { onpick }: { onpick: () => void } = $props();
  const run = (fn: () => void) => () => {
    onpick();
    fn();
  };
</script>

<button class="menu-item" role="menuitem" onclick={run(() => void copyNote())}>
  <Copy size={15} strokeWidth={1.5} />Copy this note<span class="kbd">{kbd('C', true)}</span>
</button>
<button class="menu-item" role="menuitem" onclick={run(() => void exportNote('md'))}>
  <Download size={15} strokeWidth={1.5} />Export this note<span class="kbd">{kbd('E', true)}</span>
</button>
<button class="menu-item" role="menuitemcheckbox" aria-checked={settings.value.justMyWords} onclick={run(() => void toggleJustMyWords())}>
  <EyeOff size={15} strokeWidth={1.5} />Just my words<span class="kbd">{kbd('.', true)}</span>
</button>
<div class="menu-sep"></div>
<button class="menu-item" role="menuitem" onclick={run(() => openCustomize())}>
  <Sliders size={15} strokeWidth={1.5} />Customize<span class="kbd">{kbd(',')}</span>
</button>
<button class="menu-item" role="menuitem" onclick={run(() => openCustomize('minds'))}>
  <Users size={15} strokeWidth={1.5} />Minds
</button>
