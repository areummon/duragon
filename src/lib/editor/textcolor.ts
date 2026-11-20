import { Mark } from '@tiptap/core';

export const TEXT_COLORS = ['gray', 'brown', 'red', 'orange', 'green', 'blue', 'purple'] as const;
export type TextColorName = (typeof TEXT_COLORS)[number];

declare module '@tiptap/core' {
  interface Commands<ReturnType> {
    textColor: {
      setTextColor: (color: TextColorName) => ReturnType;
      unsetTextColor: () => ReturnType;
    };
  }
}

/** Colour as a name, so it follows the light and dark themes. */
export const TextColor = Mark.create({
  name: 'textColor',
  addAttributes() {
    return {
      color: {
        default: null,
        parseHTML: (el) => el.getAttribute('data-color'),
        renderHTML: (attrs) => (attrs.color ? { 'data-color': attrs.color } : {}),
      },
    };
  },
  parseHTML: () => [{ tag: 'span[data-color]' }],
  renderHTML: ({ HTMLAttributes }) => ['span', HTMLAttributes, 0],
  addCommands() {
    return {
      setTextColor: (color) => ({ commands }) => commands.setMark(this.name, { color }),
      unsetTextColor: () => ({ commands }) => commands.unsetMark(this.name),
    };
  },
});
