import Document from '@tiptap/extension-document';
import { Node } from '@tiptap/core';

export const NoteDocument = Document.extend({ content: 'title block+' });

export const Title = Node.create({
  name: 'title',
  group: '',
  content: 'text*',
  marks: '',
  defining: true,
  isolating: true,
  parseHTML: () => [{ tag: 'h1' }],
  renderHTML: () => ['h1', { class: 'note-title' }, 0],

  addKeyboardShortcuts() {
    const inTitle = () => this.editor.state.selection.$from.parent.type === this.type;
    return {
      Enter: ({ editor }) => {
        if (!inTitle()) return false;
        const { state } = editor;
        const afterTitle = state.doc.firstChild!.nodeSize;
        return editor.chain().focus().setTextSelection(afterTitle + 1).run();
      },
      Delete: ({ editor }) => {
        const { $from, empty } = editor.state.selection;
        return empty && inTitle() && $from.parentOffset === $from.parent.content.size;
      },
      Backspace: ({ editor }) => {
        const { $from, empty } = editor.state.selection;
        return empty && $from.depth === 1 && $from.index(0) === 1 && $from.parentOffset === 0;
      },
    };
  },
});
