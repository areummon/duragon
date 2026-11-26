import { Node, type Editor } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import type { NoteMeta } from '../api/types';
import { app } from '../state/app.svelte';
import { notes } from '../state/notes.svelte';

export const noteLinkMenu = $state({
  open: false,
  query: '',
  index: 0,
  pos: { x: 0, y: 0 },
  range: { from: 0, to: 0 },
});

let dismissedAt = -1;

export function noteLinkMatches(query = noteLinkMenu.query): NoteMeta[] {
  const currentId = app.note?.id;
  const q = query.trim().toLowerCase();
  return notes.list.filter((n) => {
    if (n.id === currentId) return false;
    if (!q) return true;
    return (n.title || 'Untitled').toLowerCase().includes(q);
  });
}

function close() {
  noteLinkMenu.open = false;
  noteLinkMenu.index = 0;
}

/** Escape: closes the menu and keeps it closed while the caret stays after this `[[`. */
export function dismissNoteLink() {
  dismissedAt = noteLinkMenu.range.from;
  close();
}

export function insertNoteLink(editor: Editor, target: NoteMeta) {
  const range = { ...noteLinkMenu.range };
  close();
  editor
    .chain()
    .focus()
    .deleteRange(range)
    .insertContent([
      { type: 'noteLink', attrs: { id: target.id, title: target.title || 'Untitled' } },
      { type: 'text', text: ' ' },
    ])
    .run();
}

const key = new PluginKey('noteLink');

export const NoteLink = Node.create({
  name: 'noteLink',
  group: 'inline',
  inline: true,
  atom: true,
  selectable: true,

  addAttributes() {
    return {
      id: { default: '' },
      title: { default: '' },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'span[data-note-link]',
        getAttrs: (el) => {
          const dom = el as HTMLElement;
          return {
            id: dom.getAttribute('data-note-link') ?? '',
            title: dom.textContent ?? '',
          };
        },
      },
    ];
  },

  renderHTML({ node }) {
    const title = String(node.attrs.title || 'Untitled');
    return [
      'span',
      {
        class: 'note-link',
        'data-note-link': String(node.attrs.id ?? ''),
        role: 'link',
        tabindex: '0',
      },
      title,
    ];
  },

  renderText({ node }) {
    return String(node.attrs.title || 'Untitled');
  },

  addProseMirrorPlugins() {
    const editor = this.editor;
    return [
      new Plugin({
        key,
        view: () => ({
          update(view) {
            const { selection } = view.state;
            const { $from: head } = selection;
            const parentName = head.parent.type.name;
            if (
              !selection.empty
              || parentName === 'title'
              || parentName === 'codeBlock'
              || head.marks().some((m) => m.type.name === 'code')
            ) {
              return close();
            }
            const before = head.parent.textBetween(0, head.parentOffset, undefined, '\ufffc');
            const m = /\[\[([^\]\n\ufffc]*)$/.exec(before);
            if (!m) {
              dismissedAt = -1;
              return close();
            }
            const from = selection.from - m[1].length - 2;
            if (dismissedAt === from) return close();
            if (noteLinkMatches(m[1]).length === 0) return close();
            const coords = view.coordsAtPos(from);
            if (noteLinkMenu.query !== m[1] || !noteLinkMenu.open) noteLinkMenu.index = 0;
            const x = Math.max(12, Math.min(coords.left, window.innerWidth - 252));
            noteLinkMenu.open = true;
            noteLinkMenu.query = m[1];
            noteLinkMenu.range = { from, to: selection.from };
            noteLinkMenu.pos = { x, y: coords.bottom + 8 };
          },
          destroy: close,
        }),
        props: {
          handleKeyDown(_view, event) {
            if (!noteLinkMenu.open) return false;
            const items = noteLinkMatches();
            if (items.length === 0) return false;
            if (event.key === 'ArrowDown') {
              noteLinkMenu.index = (noteLinkMenu.index + 1) % items.length;
            } else if (event.key === 'ArrowUp') {
              noteLinkMenu.index = (noteLinkMenu.index - 1 + items.length) % items.length;
            } else if (event.key === 'Enter' || event.key === 'Tab') {
              insertNoteLink(editor, items[noteLinkMenu.index] ?? items[0]);
            } else if (event.key === 'Escape') {
              dismissNoteLink();
            } else {
              return false;
            }
            event.preventDefault();
            return true;
          },
        },
      }),
    ];
  },
});
