<script lang="ts">
  import type { Editor } from '@tiptap/core';
  import Bold from 'lucide-svelte/icons/bold';
  import Code from 'lucide-svelte/icons/code';
  import Heading1 from 'lucide-svelte/icons/heading-1';
  import Heading2 from 'lucide-svelte/icons/heading-2';
  import Heading3 from 'lucide-svelte/icons/heading-3';
  import Heading4 from 'lucide-svelte/icons/heading-4';
  import Italic from 'lucide-svelte/icons/italic';
  import List from 'lucide-svelte/icons/list';
  import ListOrdered from 'lucide-svelte/icons/list-ordered';
  import Quote from 'lucide-svelte/icons/quote';
  import SquareCode from 'lucide-svelte/icons/square-code';
  import Strikethrough from 'lucide-svelte/icons/strikethrough';
  import Type from 'lucide-svelte/icons/type';
  import Underline from 'lucide-svelte/icons/underline';
  import { tick } from 'svelte';
  import { getBackend } from '../api/backend';
  import { TEXT_COLORS } from '../editor/textcolor';
  import { selectionOccurrence } from '../editor/plain';
  import { closeOnEscape } from '../escape';
  import { isMod, isTab, kbd } from '../keys';
  import { rise } from '../motion/transitions';
  import { app } from '../state/app.svelte';
  import { MARGIN_GAP, marginWidth, selectionUi } from '../state/layout.svelte';
  import { riddlerName } from '../state/riddler';

  interface Props { editor: Editor; page: HTMLElement }
  let { editor, page }: Props = $props();

  interface Sel { from: number; to: number; text: string; top: number; left: number }
  const MENU_W = 252;

  let sel = $state<Sel | null>(null);
  // Kept apart from sel, which is already null when the menu's closing transition reads where it sat.
  let below = $state(false);
  const shown = $derived(sel !== null);
  let asking = $state(false);
  let question = $state('');
  let version = $state(0);
  let menuEl = $state<HTMLElement>();
  let input = $state<HTMLInputElement>();
  let pointer = false;

  const active = (name: string, attrs?: Record<string, unknown>) => (void version, editor.isActive(name, attrs));
  const color = () => (void version, (editor.getAttributes('textColor').color as string | undefined) ?? null);

  function hide() {
    sel = null;
    asking = false;
    question = '';
    selectionUi.pin = null;
  }

  function sync() {
    if (pointer || asking) return;
    const { from, to, empty, $from: head } = editor.state.selection;
    if (empty || head.parent.type.name === 'title' || !head.parent.isTextblock) return hide();
    const text = editor.state.doc.textBetween(from, to, '\n');
    if (!text.trim()) return hide();
    const box = page.getBoundingClientRect();
    const a = editor.view.coordsAtPos(from, 1);
    const b = editor.view.coordsAtPos(to, -1);
    const room = marginWidth() > 0;
    below = !room;
    sel = {
      from, to, text,
      top: (room ? a.top : b.bottom + 10) - box.top,
      left: room ? box.width + MARGIN_GAP : Math.max(0, Math.min(a.left - box.left, box.width - MENU_W)),
    };
  }

  $effect(() => {
    if (shown) return closeOnEscape(hide);
  });

  $effect(() => {
    const onTx = () => {
      if (sel) version++;
    };
    const onSel = () => sync();
    const ed = editor;
    ed.on('selectionUpdate', onSel);
    ed.on('transaction', onTx);
    const down = (e: MouseEvent) => {
      if (menuEl?.contains(e.target as Node)) return;
      if (ed.view.dom.contains(e.target as Node)) pointer = true;
      else if (sel) hide();
    };
    const up = () => {
      if (!pointer) return;
      pointer = false;
      requestAnimationFrame(sync);
    };
    // Taken before the editor sees it, whose own Mod-Enter would replace the selected words with a line break.
    const send = (e: KeyboardEvent) => {
      if (e.key !== 'Enter' || !isMod(e) || !sel || asking || !ed.view.hasFocus()) return;
      e.preventDefault();
      e.stopPropagation();
      void fire('riddle');
    };
    // Once the editor has passed on it: Tab goes to the toolbar ahead of the answers and marks inside the note.
    const tab = (e: KeyboardEvent) => {
      if (!isTab(e) || e.defaultPrevented || e.ctrlKey || e.altKey || e.metaKey || !sel) return;
      const first = menuEl?.querySelector('button');
      const to = e.shiftKey ? document.activeElement === first && ed.view.dom : ed.view.hasFocus() && first;
      if (!to) return;
      e.preventDefault();
      to.focus();
    };
    window.addEventListener('mousedown', down);
    window.addEventListener('mouseup', up);
    window.addEventListener('keydown', send, true);
    window.addEventListener('keydown', tab);
    return () => {
      ed.off('selectionUpdate', onSel);
      ed.off('transaction', onTx);
      window.removeEventListener('mousedown', down);
      window.removeEventListener('mouseup', up);
      window.removeEventListener('keydown', send, true);
      window.removeEventListener('keydown', tab);
    };
  });

  const keep = (e: Event) => e.preventDefault();
  const chain = () => editor.chain().focus();

  const BLOCKS = [
    { label: 'Text', icon: Type, on: () => active('paragraph') && !active('blockquote') && !active('bulletList') && !active('orderedList'), run: () => chain().setParagraph().run() },
    { label: 'Heading 1', icon: Heading1, on: () => active('heading', { level: 1 }), run: () => chain().toggleHeading({ level: 1 }).run() },
    { label: 'Heading 2', icon: Heading2, on: () => active('heading', { level: 2 }), run: () => chain().toggleHeading({ level: 2 }).run() },
    { label: 'Heading 3', icon: Heading3, on: () => active('heading', { level: 3 }), run: () => chain().toggleHeading({ level: 3 }).run() },
    { label: 'Heading 4', icon: Heading4, on: () => active('heading', { level: 4 }), run: () => chain().toggleHeading({ level: 4 }).run() },
    { label: 'Quote', icon: Quote, on: () => active('blockquote'), run: () => chain().toggleBlockquote().run() },
    { label: 'Bulleted list', icon: List, on: () => active('bulletList'), run: () => chain().toggleBulletList().run() },
    { label: 'Numbered list', icon: ListOrdered, on: () => active('orderedList'), run: () => chain().toggleOrderedList().run() },
  ];
  const MARKS = [
    { label: 'Bold', hint: kbd('B'), icon: Bold, on: () => active('bold'), run: () => chain().toggleBold().run() },
    { label: 'Italic', hint: kbd('I'), icon: Italic, on: () => active('italic'), run: () => chain().toggleItalic().run() },
    { label: 'Underline', hint: kbd('U'), icon: Underline, on: () => active('underline'), run: () => chain().toggleUnderline().run() },
    { label: 'Strikethrough', hint: '', icon: Strikethrough, on: () => active('strike'), run: () => chain().toggleStrike().run() },
    { label: 'Code', hint: '', icon: Code, on: () => active('code'), run: () => chain().toggleCode().run() },
    { label: 'Code block', hint: '', icon: SquareCode, on: () => active('codeBlock'), run: () => chain().toggleCodeBlock().run() },
  ];

  function setColor(c: string | null) {
    if (c) chain().setTextColor(c as (typeof TEXT_COLORS)[number]).run();
    else chain().unsetTextColor().run();
  }

  async function fire(kind: 'riddle' | 'ask' | 'visualize') {
    if (!sel || !app.note) return;
    const { text, from, to } = sel;
    const q = question.trim();
    const be = await getBackend();
    hide();
    editor.commands.setTextSelection(to);
    if (kind === 'visualize') void be.aiVisualize(app.note.id, text);
    else void be.aiAsk(app.note.id, text, kind === 'ask' ? q : null, selectionOccurrence(editor.state.doc, from, text));
  }

  async function startAsk() {
    if (!sel) return;
    asking = true;
    selectionUi.pin = { from: sel.from, to: sel.to };
    await tick();
    input?.focus();
  }
</script>

{#if sel}
  <div
    class="menu"
    class:below
    bind:this={menuEl}
    role="toolbar"
    tabindex="-1"
    aria-label="Selection"
    style:top="{sel.top}px"
    style:left="{sel.left}px"
    style:width="{MENU_W}px"
    transition:rise={{ y: below ? -4 : 0, scale: 0.97, duration: 160 }}
    onmousedown={(e) => e.target instanceof HTMLInputElement || keep(e)}
  >
    <div class="row">
      {#each BLOCKS as b (b.label)}
        <button class="ic" class:on={b.on()} aria-label={b.label} title={b.label} onclick={b.run}><b.icon size={16} strokeWidth={1.5} /></button>
      {/each}
    </div>
    <div class="row">
      {#each MARKS as b (b.label)}
        <button class="ic" class:on={b.on()} aria-label={b.label} title={b.hint ? `${b.label} ${b.hint}` : b.label} onclick={b.run}><b.icon size={16} strokeWidth={1.5} /></button>
      {/each}
    </div>
    <div class="row colours">
      <button class="ic a" class:on={color() === null} aria-label="Default" title="Default" onclick={() => setColor(null)}>A</button>
      {#each TEXT_COLORS as c (c)}
        <button class="ic a" class:on={color() === c} style:color="var(--text-{c})" aria-label={c} title={c[0].toUpperCase() + c.slice(1)} onclick={() => setColor(c)}>A</button>
      {/each}
    </div>
    <div class="sep"></div>
    {#if asking}
      <form class="ask" onsubmit={(e) => { e.preventDefault(); void fire('ask'); }}>
        <input bind:this={input} bind:value={question} placeholder="Ask {riddlerName()} about this…" aria-label="Your question" />
      </form>
    {:else}
      <button class="act" onclick={() => fire('riddle')}><span>{riddlerName()} this</span><span class="sub">{kbd('↵')}</span></button>
      <button class="act" onclick={startAsk}><span>Ask {riddlerName()}</span><span class="sub">Your own question</span></button>
      <button class="act" onclick={() => fire('visualize')}><span>Visualize</span><span class="sub">See it drawn out</span></button>
    {/if}
  </div>
{/if}

<style>
  .menu {
    position: absolute;
    /* Grows from the corner nearest the selection: its top left, in the margin or under the line. */
    transform-origin: top left;
    z-index: 25;
    padding: 6px;
    border: 1px solid var(--rule);
    border-radius: 12px;
    background: var(--paper);
    box-shadow: var(--lift);
    font-family: var(--font-sans);
    font-size: 14px;
    line-height: 1.3;
    letter-spacing: 0;
  }
  .row { display: flex; justify-content: space-between; gap: 1px; padding: 1px 0; }
  .ic {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 7px;
    color: var(--graphite);
    transition: background-color var(--dur-ui) var(--ease-ui), color var(--dur-ui) var(--ease-ui);
  }
  .ic:hover { background: var(--riddle-wash); color: var(--ink); }
  .ic.on { background: var(--riddle-wash); color: var(--ink); }
  .a { font-family: var(--font-sans); font-size: 15px; font-weight: 500; color: var(--ink); }
  .a.on { box-shadow: inset 0 0 0 1px var(--riddle-line); }
  .sep { height: 1px; margin: 5px 4px; background: var(--rule); }
  .act {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    width: 100%;
    padding: 7px 8px;
    border-radius: 7px;
    text-align: left;
    font-family: var(--font-voice);
    font-size: 16px;
    color: var(--riddle);
    transition: background-color var(--dur-ui) var(--ease-ui);
  }
  .act:hover { background: var(--riddle-wash); }
  .sub { font-family: var(--font-sans); font-size: 12px; color: var(--faint); }
  .ask input {
    width: 100%;
    padding: 8px;
    border: 0;
    outline: none;
    background: transparent;
    font-family: var(--font-voice);
    font-size: 16px;
    color: var(--riddle);
  }
  .ask input::placeholder { color: var(--faint); }
</style>
