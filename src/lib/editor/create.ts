import { riddlerName } from '../state/riddler';
import { Editor, Extension } from '@tiptap/core';
import Placeholder from '@tiptap/extension-placeholder';
import type { Node as PMNode } from '@tiptap/pm/model';
import { Plugin, Selection } from '@tiptap/pm/state';
import StarterKit from '@tiptap/starter-kit';
import { Caret } from './caret';
import { CodeBlock } from './codeblock';
import { MathPreview } from './math';
import { TextColor } from './textcolor';
import { SlashCommands } from './slash.svelte';
import { NoteDocument, Title } from './title';

/** Transaction meta for edits the app makes on the writer's behalf; they save but do not count as typing. */
export const QUIET = 'duragon-quiet';

export interface EditorOptions {
  element: HTMLElement;
  content: unknown;
  autofocus: 'start' | 'end' | false;
  /** quiet: the change was made for the writer (a line opened below an answer), not by them. */
  onUpdate: (editor: Editor, quiet: boolean) => void;
}

// TipTap dispatches every command a key binding runs, even one that changed nothing, and Backspace runs two of them
// before the browser deletes. Each still recomputes decorations and plugin views, so one that carries nothing stops here.
const SkipEmpty = Extension.create({
  name: 'skipEmpty',
  dispatchTransaction({ transaction: tr, next }) {
    if (tr.docChanged || tr.selectionSet || tr.storedMarksSet || tr.scrolledIntoView || !tr.isGeneric) next(tr);
  },
});

// Tab indents lists and code, and from the title it goes to the text. Anywhere else it moves focus on, as it does outside
// the note. A list item that can't indent further keeps Tab, so a press too many doesn't leave the note mid-list.
const TabKey = Extension.create({
  name: 'tabKey',
  priority: 1,
  addKeyboardShortcuts() {
    return {
      Tab: ({ editor }) => {
        const { doc, selection } = editor.state;
        if (editor.isActive('listItem')) return true;
        if (selection.$from.parent.type.name !== 'title') return false;
        return editor.commands.command(({ tr }) => {
          tr.setSelection(Selection.near(doc.resolve(doc.firstChild!.nodeSize))).scrollIntoView();
          return true;
        });
      },
    };
  },
});

const isEmptyLine = (node: PMNode) => node.type.name === 'paragraph' && node.content.size === 0;

// The note always ends with an empty line to go on writing in. Backspace on it goes up to the text and Delete before it
// does nothing, both without an edit to undo. Whatever else takes it away, such as a cut, it comes back. Empty lines
// above it delete as usual.
const LastLine = Extension.create({
  name: 'lastLine',
  priority: 1000,
  addKeyboardShortcuts() {
    const backspace = ({ editor }: { editor: Editor }) => {
      const { doc, selection } = editor.state;
      const start = doc.content.size - doc.lastChild!.nodeSize;
      const above = doc.child(doc.childCount - 2);
      if (!selection.empty || selection.from !== start + 1 || isEmptyLine(above)) return false;
      return editor.commands.command(({ tr }) => {
        tr.setSelection(Selection.near(doc.resolve(start), -1)).scrollIntoView();
        return true;
      });
    };
    const del = ({ editor }: { editor: Editor }) => {
      const { doc, selection } = editor.state;
      const { $from, empty } = selection;
      const atEnd = $from.pos + $from.depth === doc.content.size - doc.lastChild!.nodeSize;
      return empty && atEnd && $from.depth > 0 && !isEmptyLine($from.parent);
    };
    return { Backspace: backspace, 'Mod-Backspace': backspace, 'Shift-Backspace': backspace, Delete: del, 'Mod-Delete': del };
  },
  addProseMirrorPlugins() {
    return [
      new Plugin({
        appendTransaction(_transactions, _before, { doc, schema, tr }) {
          if (isEmptyLine(doc.lastChild!)) return null;
          return tr.insert(doc.content.size, schema.nodes.paragraph.create());
        },
      }),
    ];
  },
});

interface PlaceholderProps {
  editor: Editor;
  node: PMNode;
  hasAnchor: boolean;
}

// A brand-new note invites writing; any other empty line shows the hint only where the caret is.
function placeholder({ editor, node, hasAnchor }: PlaceholderProps) {
  if (node.type.name === 'title') return 'Untitled';
  const doc = editor.state.doc;
  if (doc.childCount === 2 && doc.child(1) === node && node.type.name === 'paragraph') return 'Just start writing.';
  if (!hasAnchor || node.type.name !== 'paragraph') return '';
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
        trailingNode: false,
      }),
      NoteDocument,
      TextColor,
      Title,
      CodeBlock,
      MathPreview,
      Caret,
      SlashCommands,
      Placeholder.configure({ placeholder, showOnlyCurrent: false }),
      SkipEmpty,
      TabKey,
      LastLine,
    ],
    editorProps: { attributes: { class: 'note-body', spellcheck: 'true', 'aria-label': 'Note' } },
    onUpdate: ({ editor, transaction }) => onUpdate(editor, transaction.getMeta(QUIET) === true),
  });
}
