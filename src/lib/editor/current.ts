import type { Editor } from '@tiptap/core';

/** The editor of the open note, for code outside the editor tree. */
export const current: { editor: Editor | null } = { editor: null };

export function caretParagraph(): string {
  const ed = current.editor;
  if (!ed) return '';
  const { $from } = ed.state.selection;
  return $from.parent.isTextblock ? $from.parent.textContent : '';
}
