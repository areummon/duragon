import { Extension, type Editor, type Range } from '@tiptap/core';
import Heading1 from 'lucide-svelte/icons/heading-1';
import Heading2 from 'lucide-svelte/icons/heading-2';
import Heading3 from 'lucide-svelte/icons/heading-3';
import Heading4 from 'lucide-svelte/icons/heading-4';
import List from 'lucide-svelte/icons/list';
import ListOrdered from 'lucide-svelte/icons/list-ordered';
import Minus from 'lucide-svelte/icons/minus';
import SquareCode from 'lucide-svelte/icons/square-code';
import TextQuote from 'lucide-svelte/icons/text-quote';
import Type from 'lucide-svelte/icons/type';
import { createInlineMenu } from './inline-menu.svelte';

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

export const slashMenu = createInlineMenu<SlashItem>({
  key: 'slash',
  width: 220,
  trigger: /(?:^|\s)\/([\w ]*)$/,
  triggerLength: 1,
  canOpen: (head) => head.parent.type.name === 'paragraph',
  matches: (query) => {
    const q = query.trim().toLowerCase();
    return SLASH_ITEMS.filter((i) => i.label.toLowerCase().includes(q));
  },
  select: (editor, item, range) => item.run(editor, range),
});

export const SlashCommands = Extension.create({
  name: 'slashCommands',

  addProseMirrorPlugins() {
    return [slashMenu.createPlugin(this.editor)];
  },
});
