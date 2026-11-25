<script lang="ts">
  import { tick } from 'svelte';
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
  import TrashIcon from 'lucide-svelte/icons/trash-2';
  import type { DeleteMode, Folder, NoteMeta } from '../api/types';
  import { app, deleteNote, moveNote, newNote, openNote, peekSidebar, setSidebar, unpeekSidebar } from '../state/app.svelte';
  import { createFolder, deleteFolder, folderWithin, moveFolder, notes, renameFolder } from '../state/notes.svelte';
  import { layout } from '../state/layout.svelte';
  import { settings } from '../state/settings.svelte';
  import { closeOnEscape } from '../escape';
  import { focusLost } from '../focus';
  import { kbd } from '../keys';
  import { unfold, veil } from '../motion/transitions';
  import Blot from './Blot.svelte';
  import Popover from './Popover.svelte';
  import SettingsMenu from './SettingsMenu.svelte';

  const recent = $derived(notes.list.filter((n) => n.folderId === null));
  let showAll = $state(false);
  const shown = $derived(showAll ? recent : recent.slice(0, 10));
  const inFolder = (f: Folder) => notes.list.filter((n) => n.folderId === f.id);
  const roots = $derived(notes.folders.filter((f) => f.parentId === null));
  const subfolders = (f: Folder) => notes.folders.filter((c) => c.parentId === f.id);

  let collapsed = $state<Record<string, boolean>>({});
  let noteMenu = $state<{ id: string; at: { x: number; y: number }; view: 'main' | 'move' | 'confirm'; mode: DeleteMode } | null>(null);
  let folderMenu = $state<{ id: string; at: { x: number; y: number }; confirm: boolean } | null>(null);
  let editing = $state<{ id: string | null; name: string } | null>(null);
  let settingsOpen = $state(false);
  let list: HTMLElement;

  $effect(() => {
    if (app.sidebarOpen) return closeOnEscape(() => setSidebar(false));
  });

  function openNoteMenu(e: MouseEvent, n: NoteMeta) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    folderMenu = null;
    noteMenu = { id: n.id, at: { x: r.left, y: r.bottom + 4 }, view: 'main', mode: settings.value.deletedNotes };
  }
  function openFolderMenu(e: MouseEvent, f: Folder) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    noteMenu = null;
    folderMenu = { id: f.id, at: { x: r.left, y: r.bottom + 4 }, confirm: false };
  }

  function remove(menu: NonNullable<typeof noteMenu>) {
    if (menu.mode === 'permanent' && menu.view !== 'confirm') {
      menu.view = 'confirm';
      return;
    }
    const rows = () => [...list.querySelectorAll<HTMLElement>('[data-note]')];
    const at = rows().findIndex((r) => r.dataset.note === menu.id);
    void deleteNote(menu.id, menu.mode).then(async () => {
      await tick();
      // Focus that went with the row goes to the one that took its place.
      if (focusLost()) (rows()[at] ?? rows().at(-1))?.querySelector<HTMLElement>('.open')?.focus();
    });
    noteMenu = null;
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

  // Dragging a note or folder onto a folder files it there; dropping it anywhere else in the list
  // takes it to the top level. `data-drop` names the folder under the pointer, and '' is the top level.
  type Dragged = { kind: 'note' | 'folder'; id: string; label: string; from: string | null };
  let press: { item: Dragged; x: number; y: number } | null = null;
  let drag = $state<{ item: Dragged; x: number; y: number; over: string | undefined } | null>(null);
  let justDragged = false;

  function grab(e: PointerEvent, item: Dragged) {
    if (e.button === 0 && e.pointerType !== 'touch') press = { item, x: e.clientX, y: e.clientY };
  }

  const accepts = (item: Dragged, target: string) => {
    const to = target || null;
    if (to === item.from) return false;
    return item.kind === 'note' || to === null || !folderWithin(to, item.id);
  };

  function track(e: PointerEvent) {
    if (!press) return;
    if (!drag && Math.hypot(e.clientX - press.x, e.clientY - press.y) < 4) return;
    const zone = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-drop]');
    const target = zone?.dataset.drop;
    drag = { item: press.item, x: e.clientX, y: e.clientY, over: target !== undefined && accepts(press.item, target) ? target : undefined };
  }

  function drop() {
    const d = drag;
    press = null;
    drag = null;
    if (!d) return;
    justDragged = true;
    if (d.over === undefined) return;
    const to = d.over || null;
    if (to) collapsed[to] = false;
    if (d.item.kind === 'note') void moveNote(d.item.id, to);
    else void moveFolder(d.item.id, to);
  }

  function cancel(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !drag) return;
    e.stopPropagation();
    press = null;
    drag = null;
    justDragged = true;
  }

  function swallowClick(e: MouseEvent) {
    if (!justDragged) return;
    justDragged = false;
    // A press from the keyboard is not the click the drag ended with.
    if (e.detail === 0) return;
    e.stopPropagation();
    e.preventDefault();
  }

  const busy = () =>
    drag !== null || noteMenu !== null || folderMenu !== null || settingsOpen || editing !== null ||
    app.paletteOpen || app.customizeOpen || app.trashOpen || app.mindEditor !== null;

  // Beside the edge, resting on the empty page opens a peek too. Nothing lies over the page there, so its threads and
  // words keep the pointer. Once it has opened or been closed, the pointer has to leave and come back.
  const NEAR = 32;
  const REST_MS = 120;
  let rest: number | undefined;
  let armed = true;

  function leaveEdge() {
    clearTimeout(rest);
    rest = undefined;
  }

  /** A press there is meant for the page. */
  function pressEdge(e: PointerEvent) {
    leaveEdge();
    if (e.clientX < NEAR) armed = false;
  }

  function nearEdge(e: PointerEvent) {
    if (e.clientX >= NEAR) {
      armed = true;
      return leaveEdge();
    }
    if (app.sidebar !== 'closed') armed = false;
    if (!armed || e.buttons || e.pointerType === 'touch' || layout.vw < 768) return leaveEdge();
    const empty = e.target instanceof Element && e.target.matches('.stage, body, html');
    if (!empty) return leaveEdge();
    rest ??= window.setTimeout(() => {
      leaveEdge();
      armed = false;
      peekSidebar();
    }, REST_MS);
  }

  // Leaving towards the window's left edge keeps a peek open, as does any menu or dialog in use.
  function leave(e: MouseEvent) {
    if (e.clientX <= 40 || busy()) return;
    armed = true;
    unpeekSidebar();
  }

  const focusInput = (node: HTMLInputElement) => {
    node.focus();
    node.select();
  };
</script>

<svelte:window onpointermove={(e) => { track(e); nearEdge(e); }} onpointerdown={pressEdge} onpointerout={(e) => { if (!e.relatedTarget) leaveEdge(); }} onpointerup={drop} onkeydowncapture={cancel} />

<div class="edge" aria-hidden="true" onmouseenter={peekSidebar}></div>
{#if app.sidebarOpen}
  <div class="scrim" role="presentation" onclick={() => setSidebar(false)} onmouseenter={leave}></div>
{/if}

<nav class="sidebar" class:open={app.sidebarOpen} class:dragging={drag !== null} inert={!app.sidebarOpen} aria-label="Notes" onmouseleave={leave}>
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

  <div class="list" bind:this={list} class:drop-root={drag?.over === ''} data-drop="" onpointerdowncapture={() => (justDragged = false)} onclickcapture={swallowClick}>
    {#snippet noteRow(n: NoteMeta)}
      <div class="row note" data-note={n.id} class:current={app.note?.id === n.id} class:lifted={drag?.item.id === n.id}>
        <button
          class="open"
          onpointerdown={(e) => grab(e, { kind: 'note', id: n.id, label: n.title || 'Untitled', from: n.folderId })}
          onclick={() => openNote(n.id)}
        >
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

    {#snippet folderTree(f: Folder)}
      <div class="row folder" class:drop={drag?.over === f.id} class:lifted={drag?.item.id === f.id} data-drop={f.id}>
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
          <button
            class="open"
            aria-expanded={!collapsed[f.id]}
            onpointerdown={(e) => grab(e, { kind: 'folder', id: f.id, label: f.name, from: f.parentId })}
            onclick={() => (collapsed[f.id] = !collapsed[f.id])}>
            {#if collapsed[f.id]}<ChevronRight size={15} strokeWidth={1.5} />{:else}<ChevronDown size={15} strokeWidth={1.5} />{/if}
            <span class="title">{f.name}</span>
          </button>
          <button class="more" class:shown={folderMenu?.id === f.id} aria-label="Folder options" onclick={(e) => openFolderMenu(e, f)}>
            <MoreHorizontal size={15} strokeWidth={1.5} />
          </button>
        {/if}
      </div>
      {#if !collapsed[f.id]}
        <div class="children" data-drop={f.id} transition:unfold={{ duration: 200 }}>
          {#each subfolders(f) as c (c.id)}
            {@render folderTree(c)}
          {/each}
          {#each inFolder(f) as n (n.id)}
            {@render noteRow(n)}
          {/each}
        </div>
      {/if}
    {/snippet}

    {#each roots as f (f.id)}
      {@render folderTree(f)}
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
    <button class="row action" aria-haspopup="dialog" onclick={() => (app.trashOpen = true)}>
      <TrashIcon size={15} strokeWidth={1.5} /><span>Trash</span>
    </button>
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

{#if drag}
  <div class="ghost" aria-hidden="true" style:transform="translate({drag.x + 14}px, {drag.y + 10}px)">
    {#if drag.item.kind === 'folder'}<FolderIcon size={15} strokeWidth={1.5} />{:else}<FileText size={15} strokeWidth={1.5} />{/if}
    <span class="title">{drag.item.label}</span>
  </div>
{/if}

<Popover open={noteMenu !== null} onclose={() => (noteMenu = null)} at={noteMenu?.at} width={200} label="Note options">
  {#if noteMenu}
    {@const id = noteMenu.id}
    {#key noteMenu.view}
      <div class="view" in:veil={{ duration: 150 }}>
        {#if noteMenu.view === 'main'}
          <button class="menu-item" role="menuitem" onclick={() => renameNote(id)}>Rename</button>
          <button class="menu-item" role="menuitem" onclick={() => noteMenu && (noteMenu.view = 'move')}>Move to folder</button>
          <button class="menu-item" role="menuitem" onclick={() => noteMenu && remove(noteMenu)}>Delete</button>
        {:else if noteMenu.view === 'move'}
          <button class="menu-item" role="menuitem" onclick={() => { moveNote(id, null); noteMenu = null; }}>No folder</button>
          {#each notes.folders as f (f.id)}
            <button class="menu-item" role="menuitem" onclick={() => { moveNote(id, f.id); noteMenu = null; }}>{f.name}</button>
          {/each}
          {#if notes.folders.length === 0}<div class="empty">Make a folder first.</div>{/if}
        {:else}
          <div class="empty">Delete this note for good?</div>
          <button class="menu-item danger" role="menuitem" onclick={() => noteMenu && remove(noteMenu)}>Delete</button>
          <button class="menu-item" role="menuitem" onclick={() => (noteMenu = null)}>Keep it</button>
        {/if}
      </div>
    {/key}
  {/if}
</Popover>

<Popover open={folderMenu !== null} onclose={() => (folderMenu = null)} at={folderMenu?.at} width={200} label="Folder options">
  {#if folderMenu}
    {@const id = folderMenu.id}
    {#key folderMenu.confirm}
      <div class="view" in:veil={{ duration: 150 }}>
        {#if !folderMenu.confirm}
          <button class="menu-item" role="menuitem" onclick={() => { editing = { id, name: notes.folders.find((f) => f.id === id)?.name ?? '' }; folderMenu = null; }}>Rename</button>
          <button class="menu-item" role="menuitem" onclick={() => folderMenu && (folderMenu.confirm = true)}>Delete folder</button>
        {:else}
          <div class="empty">What's inside moves up a level.</div>
          <button class="menu-item danger" role="menuitem" onclick={() => { deleteFolder(id); folderMenu = null; }}>Delete folder</button>
          <button class="menu-item" role="menuitem" onclick={() => (folderMenu = null)}>Keep it</button>
        {/if}
      </div>
    {/key}
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
  .foot .row { height: 40px; }
  .settings span { flex: 1; }
  .empty { padding: 6px 10px; font-size: 13px; color: var(--faint); }
  .danger { color: var(--text-red); }
  .view { display: flex; flex-direction: column; }

  .dragging, .dragging * { cursor: grabbing; user-select: none; }
  .dragging .row:hover:not(.drop) { background: none; }
  .dragging .more { opacity: 0; }
  .lifted { opacity: 0.4; }
  .folder.drop { background: var(--riddle-wash); color: var(--ink); box-shadow: inset 0 0 0 1px var(--riddle-line); }
  .list.drop-root { border-radius: 8px; box-shadow: inset 0 0 0 1px var(--riddle-line); }
  .ghost {
    position: fixed;
    top: 0;
    left: 0;
    z-index: 40;
    display: flex;
    align-items: center;
    gap: 8px;
    max-width: 220px;
    height: 28px;
    padding: 0 10px;
    border-radius: 8px;
    background: var(--paper);
    box-shadow: 0 0 0 1px var(--rule), 0 6px 18px rgb(0 0 0 / 0.12);
    font-size: 13px;
    color: var(--ink);
    pointer-events: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .sidebar { transition: opacity var(--dur-drawer) var(--ease-drawer), visibility 0s linear var(--dur-drawer); }
    .sidebar.open { transition-delay: 0s; }
  }
</style>
