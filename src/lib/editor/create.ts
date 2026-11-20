import { riddlerName } from '../state/riddler';
import { Editor } from '@tiptap/core';
import Placeholder from '@tiptap/extension-placeholder';
import StarterKit from '@tiptap/starter-kit';
import { CodeBlock } from './codeblock';
import { TextColor } from './textcolor';
import { SlashCommands } from './slash.svelte';
import { NoteDocument, Title } from './title';

export interface EditorOptions {
  element: HTMLElement;
  content: unknown;
  autofocus: 'start' | 'end' | false;
  onUpdate: (editor: Editor) => void;
}

function placeholder({ node, pos, editor }: { node: { type: { name: string } }; pos: number; editor: Editor }) {
  const doc = editor.state.doc;
  if (node.type.name === 'title') return 'Untitled';
  if (node.type.name !== 'paragraph') return '';
  const index = doc.resolve(pos).index(0);
  if (index === 1) return `Write, talk to ${riddlerName()}, or type / for commands…`;
  return index === doc.childCount - 1 ? 'Just start writing.' : '';
}

export function createEditor({ element, content, autofocus, onUpdate }: EditorOptions) {
  return new Editor({
    element,
    content: content as object,
    autofocus,
    extensions: [
      StarterKit.configure({
        document: false,
        codeBlock: false,
        heading: { levels: [1, 2, 3, 4] },
        link: false,
      }),
      NoteDocument,
      TextColor,
      Title,
      CodeBlock,
      SlashCommands,
      Placeholder.configure({ placeholder, showOnlyCurrent: false }),
    ],
    editorProps: { attributes: { class: 'note-body', spellcheck: 'true', 'aria-label': 'Note' } },
    onUpdate: ({ editor }) => onUpdate(editor),
  });
}
