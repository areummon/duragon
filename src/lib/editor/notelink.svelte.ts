import { Node } from '@tiptap/core';
import type { NoteMeta } from '../api/types';
import { app } from '../state/app.svelte';
import { notes } from '../state/notes.svelte';
import { createInlineMenu } from './inline-menu.svelte';

export const noteLinkMenu = createInlineMenu<NoteMeta>({
  key: 'noteLink',
  width: 240,
  trigger: /\[\[([^\]\n\ufffc]*)$/,
  triggerLength: 2,
  canOpen: (head) => {
    const parentName = head.parent.type.name;
    return (
      parentName !== 'title'
      && parentName !== 'codeBlock'
      && !head.marks().some((m) => m.type.name === 'code')
    );
  },
  matches: (query) => {
    const currentId = app.note?.id;
    const q = query.trim().toLowerCase();
    return notes.list.filter((n) => {
      if (n.id === currentId) return false;
      if (!q) return true;
      return (n.title || 'Untitled').toLowerCase().includes(q);
    });
  },
  select: (editor, target, range) => {
    editor
      .chain()
      .focus()
      .deleteRange(range)
      .insertContent([
        { type: 'noteLink', attrs: { id: target.id, title: target.title || 'Untitled' } },
        { type: 'text', text: ' ' },
      ])
      .run();
  },
});

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
          if (typeof el === 'string') return false;
          return {
            id: el.getAttribute('data-note-link') ?? '',
            title: el.textContent ?? '',
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
    return [noteLinkMenu.createPlugin(this.editor)];
  },
});
