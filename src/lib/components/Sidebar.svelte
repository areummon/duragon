<script lang="ts">
  import ChevronDown from 'lucide-svelte/icons/chevron-down';
  import ChevronRight from 'lucide-svelte/icons/chevron-right';
  import FileText from 'lucide-svelte/icons/file-text';
  import FolderIcon from 'lucide-svelte/icons/folder';
  import FolderPlus from 'lucide-svelte/icons/folder-plus';
  import Library from 'lucide-svelte/icons/library';
  import MoreHorizontal from 'lucide-svelte/icons/more-horizontal';
  import Plus from 'lucide-svelte/icons/plus';
  import Search from 'lucide-svelte/icons/search';
  import SettingsIcon from 'lucide-svelte/icons/settings';
  import type { Folder, NoteMeta } from '../api/types';
  import { app, deleteNote, moveNote, newNote, openNote, peekSidebar, setSidebar, unpeekSidebar } from '../state/app.svelte';
  import { createFolder, deleteFolder, notes, renameFolder } from '../state/notes.svelte';
  import { kbd } from '../keys';
  import { unfold } from '../motion/transitions';
  import Blot from './Blot.svelte';
  import Popover from './Popover.svelte';
  import SettingsMenu from './SettingsMenu.svelte';

  const recent = $derived(notes.list.filter((n) => n.folderId === null));
  let showAll = $state(false);
  const shown = $derived(showAll ? recent : recent.slice(0, 10));
  const inFolder = (f: Folder) => notes.list.filter((n) => n.folderId === f.id);

  let collapsed = $state<Record<string, boolean>>({});
  let noteMenu = $state<{ id: string; at: { x: number; y: number }; view: 'main' | 'move' | 'confirm' } | null>(null);
  let folderMenu = $state<{ id: string; at: { x: number; y: number }; confirm: boolean } | null>(null);
  let editing = $state<{ id: string | null; name: string } | null>(null);
  let settingsOpen = $state(false);

  function openNoteMenu(e: MouseEvent, n: NoteMeta) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    folderMenu = null;
    noteMenu = { id: n.id, at: { x: r.left, y: r.bottom + 4 }, view: 'main' };
  }
  function openFolderMenu(e: MouseEvent, f: Folder) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    noteMenu = null;
    folderMenu = { id: f.id, at: { x: r.left, y: r.bottom + 4 }, confirm: false };
  }

  async function renameNote(id: string) {
    noteMenu = null;
    await openNote(id);
    app.focusTitle = true;
  }

  async function commitFolder(e: { id: string | null; name: string }) {
    editing = null;
    const name = e.name.trim();
    if (e.id) {
      if (name) await renameFolder(e.id, name);
    } else await createFolder(name || 'New folder');
  }

  const busy = () =>
    noteMenu !== null || folderMenu !== null || settingsOpen || editing !== null ||
    app.paletteOpen || app.customizeOpen || app.mindEditor !== null;

  // Leaving towards the window's left edge keeps a peek open, as does any menu or dialog in use.
  function leave(e: MouseEvent) {
    if (e.clientX > 40 && !busy()) unpeekSidebar();
  }

  const focusInput = (node: HTMLInputElement) => {
    node.focus();
    node.select();
  };
</script>

<div class="edge" aria-hidden="true" onmouseenter={peekSidebar}></div>
{#if app.sidebarOpen}
  <div class="scrim" role="presentation" onclick={() => setSidebar(false)} onmouseenter={leave}></div>
{/if}

<nav class="sidebar" class:open={app.sidebarOpen} inert={!app.sidebarOpen} aria-label="Notes" onmouseleave={leave}>
  <div class="top">
    <button class="row action" onclick={() => (app.paletteOpen = true)}>
      <Search size={15} strokeWidth={1.5} /><span>Find or start a note</span><span class="kbd">{kbd('K')}</span>
    </button>
    <button class="row action" onclick={() => newNote()}>
      <Plus size={15} strokeWidth={1.5} /><span>New note</span>
    </button>
    <button class="row action" aria-pressed={showAll} onclick={() => (showAll = !showAll)}>
      <Library size={15} strokeWidth={1.5} /><span>All notes</span><span class="count">{notes.list.length}</span>
    </button>
  </div>

  <div class="list">
    {#snippet noteRow(n: NoteMeta)}
      <div class="row note" class:current={app.note?.id === n.id}>
        <button class="open" onclick={() => openNote(n.id)}>
          <FileText size={15} strokeWidth={1.5} />
          <span class="title">{n.title || 'Untitled'}</span>
          {#if n.hasOpenQuestion}<Blot size={9} />{/if}
        </button>
        <button class="more" class:shown={noteMenu?.id === n.id} aria-label="Note options" onclick={(e) => openNoteMenu(e, n)}>
          <MoreHorizontal size={15} strokeWidth={1.5} />
        </button>
      </div>
    {/snippet}

    {#each shown as n (n.id)}
      {@render noteRow(n)}
    {/each}

    <div class="gap"></div>

    {#each notes.folders as f (f.id)}
      <div class="row folder">
        {#if editing?.id === f.id}
          <div class="open">
            <FolderIcon size={15} strokeWidth={1.5} />
            <input
              class="name"
              bind:value={editing.name}
              use:focusInput
              aria-label="Folder name"
              onblur={() => editing && commitFolder(editing)}
              onkeydown={(e) => {
                if (e.key === 'Enter') (e.currentTarget as HTMLInputElement).blur();
                if (e.key === 'Escape') { e.stopPropagation(); editing = null; }
              }}
            />
          </div>
        {:else}
          <button class="open" aria-expanded={!collapsed[f.id]} onclick={() => (collapsed[f.id] = !collapsed[f.id])}>
            {#if collapsed[f.id]}<ChevronRight size={15} strokeWidth={1.5} />{:else}<ChevronDown size={15} strokeWidth={1.5} />{/if}
            <span class="title">{f.name}</span>
          </button>
          <button class="more" class:shown={folderMenu?.id === f.id} aria-label="Folder options" onclick={(e) => openFolderMenu(e, f)}>
            <MoreHorizontal size={15} strokeWidth={1.5} />
          </button>
        {/if}
      </div>
      {#if !collapsed[f.id]}
        <div class="children" transition:unfold={{ duration: 200 }}>
          {#each inFolder(f) as n (n.id)}
            {@render noteRow(n)}
          {/each}
        </div>
      {/if}
    {/each}

    {#if editing && editing.id === null}
      <div class="row folder">
        <div class="open">
          <FolderIcon size={15} strokeWidth={1.5} />
          <input
            class="name"
            placeholder="Folder name"
            bind:value={editing.name}
            use:focusInput
            aria-label="Folder name"
            onblur={() => editing && commitFolder(editing)}
            onkeydown={(e) => {
              if (e.key === 'Enter') (e.currentTarget as HTMLInputElement).blur();
              if (e.key === 'Escape') { e.stopPropagation(); editing = null; }
            }}
          />
        </div>
      </div>
    {:else}
      <button class="row action" onclick={() => (editing = { id: null, name: '' })}>
        <FolderPlus size={15} strokeWidth={1.5} /><span>New folder</span>
      </button>
    {/if}
  </div>

  <div class="foot">
    <div class="host">
      <button class="row settings" aria-haspopup="menu" aria-expanded={settingsOpen} onclick={() => (settingsOpen = !settingsOpen)}>
        <SettingsIcon size={15} strokeWidth={1.5} /><span>Settings</span><ChevronDown size={15} strokeWidth={1.5} />
      </button>
      <Popover open={settingsOpen} onclose={() => (settingsOpen = false)} up width={248} label="Settings">
        <SettingsMenu onpick={() => (settingsOpen = false)} />
      </Popover>
    </div>
  </div>
</nav>

<Popover open={noteMenu !== null} onclose={() => (noteMenu = null)} at={noteMenu?.at} width={200} label="Note options">
  {#if noteMenu}
    {@const id = noteMenu.id}
    {#if noteMenu.view === 'main'}
      <button class="menu-item" role="menuitem" onclick={() => renameNote(id)}>Rename</button>
      <button class="menu-item" role="menuitem" onclick={() => noteMenu && (noteMenu.view = 'move')}>Move to folder</button>
      <button class="menu-item" role="menuitem" onclick={() => noteMenu && (noteMenu.view = 'confirm')}>Delete</button>
    {:else if noteMenu.view === 'move'}
      <button class="menu-item" role="menuitem" onclick={() => { moveNote(id, null); noteMenu = null; }}>No folder</button>
      {#each notes.folders as f (f.id)}
        <button class="menu-item" role="menuitem" onclick={() => { moveNote(id, f.id); noteMenu = null; }}>{f.name}</button>
      {/each}
      {#if notes.folders.length === 0}<div class="empty">Make a folder first.</div>{/if}
    {:else}
      <div class="empty">Delete this note for good?</div>
      <button class="menu-item danger" role="menuitem" onclick={() => { deleteNote(id); noteMenu = null; }}>Delete</button>
      <button class="menu-item" role="menuitem" onclick={() => (noteMenu = null)}>Keep it</button>
    {/if}
  {/if}
</Popover>

<Popover open={folderMenu !== null} onclose={() => (folderMenu = null)} at={folderMenu?.at} width={200} label="Folder options">
  {#if folderMenu}
    {@const id = folderMenu.id}
    {#if !folderMenu.confirm}
      <button class="menu-item" role="menuitem" onclick={() => { editing = { id, name: notes.folders.find((f) => f.id === id)?.name ?? '' }; folderMenu = null; }}>Rename</button>
      <button class="menu-item" role="menuitem" onclick={() => folderMenu && (folderMenu.confirm = true)}>Delete folder</button>
    {:else}
      <div class="empty">Notes inside stay, without a folder.</div>
      <button class="menu-item danger" role="menuitem" onclick={() => { deleteFolder(id); folderMenu = null; }}>Delete folder</button>
      <button class="menu-item" role="menuitem" onclick={() => (folderMenu = null)}>Keep it</button>
    {/if}
  {/if}
</Popover>

<style>
  .sidebar {
    position: fixed;
    inset: 0 auto 0 0;
    z-index: 10;
    width: 288px;
    display: flex;
    flex-direction: column;
    padding: 79px 12px 12px;
    background: var(--paper);
    max-width: 78vw;
    opacity: 0;
    transform: translateX(-24px);
    visibility: hidden;
    transition:
      transform var(--dur-drawer) var(--ease-drawer),
      opacity var(--dur-drawer) var(--ease-drawer),
      visibility 0s linear var(--dur-drawer);
  }
  .sidebar.open { opacity: 1; transform: none; visibility: visible; transition-delay: 0s; }

  .edge { display: none; position: fixed; inset: 0 auto 0 0; width: 8px; z-index: 30; }
  @media (min-width: 768px) {
    .edge { display: block; }
  }
  .scrim { position: fixed; inset: 0 0 0 min(288px, 78vw); z-index: 9; }

  .top { display: flex; flex-direction: column; gap: 8px; }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 10px;
    border-radius: 8px;
    font-size: 14px;
    color: var(--graphite);
    text-align: left;
    width: 100%;
  }
  .row :global(svg) { flex: none; }
  .action { transition: color var(--dur-ui) var(--ease-ui), background-color var(--dur-ui) var(--ease-ui); }
  .action:hover, .settings:hover { color: var(--ink); }
  .action span:first-of-type { flex: 1; text-align: left; }
  .kbd, .count { margin-left: auto; font-size: 12px; color: var(--faint); letter-spacing: 0.02em; }

  .list { flex: 1; min-height: 0; overflow-y: auto; overscroll-behavior: contain; margin-top: 27px; display: flex; flex-direction: column; gap: 2px; scrollbar-width: none; }
  .gap { height: 14px; flex: none; }
  .note, .folder { position: relative; padding: 0; gap: 0; }
  .note .open, .folder .open {
    flex: 1;
    min-width: 0;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px;
    border-radius: 8px;
    text-align: left;
  }
  .title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .note:hover, .folder:hover { background: var(--riddle-wash); }
  .note.current { background: var(--riddle-wash); color: var(--ink); }
  .note.current .title { color: var(--ink); }
  .more {
    position: absolute;
    right: 4px;
    top: 4px;
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 6px;
    color: var(--faint);
    opacity: 0;
    transition: opacity var(--dur-ui) var(--ease-ui);
  }
  .row:hover .more, .more:focus-visible, .more.shown { opacity: 1; }
  .more:hover { color: var(--ink); }
  .children { padding-left: 14px; display: flex; flex-direction: column; gap: 2px; }
  .name {
    flex: 1;
    min-width: 0;
    height: 24px;
    padding: 0 6px;
    margin-left: -6px;
    border: 1px solid var(--rule);
    border-radius: 6px;
    background: transparent;
    font: inherit;
    color: var(--ink);
    outline: none;
  }
  .name:focus { border-color: var(--riddle-line); }

  .foot { border-top: 1px solid var(--rule); padding: 10px 0 0; }
  .host { position: relative; }
  .settings { height: 40px; }
  .settings span { flex: 1; }
  .empty { padding: 6px 10px; font-size: 13px; color: var(--faint); }
  .danger { color: var(--text-red); }
</style>
