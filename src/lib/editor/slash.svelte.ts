import { Extension, type Editor, type Range } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import {
  Heading1, Heading2, Heading3, Heading4, List, ListOrdered, Minus, SquareCode, TextQuote, Type,
} from 'lucide-svelte';

export interface SlashItem {
  id: string;
  label: string;
  icon: typeof Type;
  run: (editor: Editor, range: Range) => void;
}

const block = (editor: Editor, range: Range) => editor.chain().focus().deleteRange(range);

export const SLASH_ITEMS: SlashItem[] = [
  { id: 'text', label: 'Text', icon: Type, run: (e, r) => block(e, r).setParagraph().run() },
  { id: 'h1', label: 'Heading 1', icon: Heading1, run: (e, r) => block(e, r).setHeading({ level: 1 }).run() },
  { id: 'h2', label: 'Heading 2', icon: Heading2, run: (e, r) => block(e, r).setHeading({ level: 2 }).run() },
  { id: 'h3', label: 'Heading 3', icon: Heading3, run: (e, r) => block(e, r).setHeading({ level: 3 }).run() },
  { id: 'h4', label: 'Heading 4', icon: Heading4, run: (e, r) => block(e, r).setHeading({ level: 4 }).run() },
  { id: 'ul', label: 'Bulleted list', icon: List, run: (e, r) => block(e, r).toggleBulletList().run() },
  { id: 'ol', label: 'Numbered list', icon: ListOrdered, run: (e, r) => block(e, r).toggleOrderedList().run() },
  { id: 'quote', label: 'Quote', icon: TextQuote, run: (e, r) => block(e, r).toggleBlockquote().run() },
  { id: 'code', label: 'Code block', icon: SquareCode, run: (e, r) => block(e, r).setCodeBlock().run() },
  { id: 'hr', label: 'Divider', icon: Minus, run: (e, r) => block(e, r).setHorizontalRule().run() },
];

export const slash = $state({
  open: false,
  query: '',
  from: 0,
  to: 0,
  index: 0,
  x: 0,
  y: 0,
});

let dismissedAt = -1;

export function slashMatches(query = slash.query): SlashItem[] {
  const q = query.trim().toLowerCase();
  return SLASH_ITEMS.filter((i) => i.label.toLowerCase().includes(q));
}

function close() {
  slash.open = false;
  slash.index = 0;
}

export function runSlash(editor: Editor, item: SlashItem) {
  const range = { from: slash.from, to: slash.to };
  close();
  item.run(editor, range);
}

const key = new PluginKey('slash');

export const SlashCommands = Extension.create({
  name: 'slashCommands',

  addProseMirrorPlugins() {
    const editor = this.editor;
    return [
      new Plugin({
        key,
        view: () => ({
          update(view) {
            const { selection } = view.state;
            const { $from: head } = selection;
            if (!selection.empty || head.parent.type.name !== 'paragraph') return close();
            const before = head.parent.textBetween(0, head.parentOffset, undefined, '￼');
            const m = /(?:^|\s)\/([\w ]*)$/.exec(before);
            if (!m) {
              dismissedAt = -1;
              return close();
            }
            const from = selection.from - m[1].length - 1;
            if (dismissedAt === from) return close();
            if (slashMatches(m[1]).length === 0) return close();
            const coords = view.coordsAtPos(from);
            if (slash.query !== m[1] || !slash.open) slash.index = 0;
            Object.assign(slash, {
              open: true, query: m[1], from, to: selection.from, x: coords.left, y: coords.bottom + 8,
            });
          },
          destroy: close,
        }),
        props: {
          handleKeyDown(_view, event) {
            if (!slash.open) return false;
            const items = slashMatches();
            if (event.key === 'ArrowDown') slash.index = (slash.index + 1) % items.length;
            else if (event.key === 'ArrowUp') slash.index = (slash.index - 1 + items.length) % items.length;
            else if (event.key === 'Enter' || event.key === 'Tab') runSlash(editor, items[slash.index]);
            else if (event.key === 'Escape') {
              dismissedAt = slash.from;
              close();
            } else return false;
            event.preventDefault();
            return true;
          },
        },
      }),
    ];
  },
});
