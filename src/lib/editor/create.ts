import { riddlerName } from '../state/riddler';
import { Editor } from '@tiptap/core';
import Placeholder from '@tiptap/extension-placeholder';
import type { Node as PMNode } from '@tiptap/pm/model';
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

interface PlaceholderProps {
  editor: Editor;
  node: PMNode;
  pos: number;
  hasAnchor: boolean;
}

// A brand-new note invites writing; any other empty line shows the hint only where the caret is.
function placeholder({ editor, node, pos, hasAnchor }: PlaceholderProps) {
  if (node.type.name === 'title') return 'Untitled';
  const doc = editor.state.doc;
  if (doc.childCount === 2 && doc.child(1) === node && node.type.name === 'paragraph') return 'Just start writing.';
  if (!hasAnchor || node.type.name !== 'paragraph') return '';
  if (doc.resolve(pos).parent.type.name === 'listItem') return 'List';
  return `Write, talk to ${riddlerName()}, or type / for commands…`;
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
