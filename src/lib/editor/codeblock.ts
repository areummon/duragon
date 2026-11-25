import type { Editor } from '@tiptap/core';
import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight';
import { COPY_ICON, copyFrom } from '../copy';
import { COMMON_LANGUAGES, ensureLowlight, isLowlightReady, lowlight } from '../rich';
import { QUIET } from './create';

/** Enter on a last line of ``` ends the block, as a closing fence does in Markdown, and the caret goes on below it. */
function closeFence(editor: Editor): boolean {
  const { $from, empty } = editor.state.selection;
  const block = $from.parent;
  if (!empty || block.type.name !== 'codeBlock' || $from.parentOffset !== block.content.size) return false;
  const text = block.textContent;
  if (text !== '```' && !text.endsWith('\n```')) return false;
  const fence = text === '```' ? 3 : 4;
  return editor.chain().deleteRange({ from: $from.pos - fence, to: $from.pos }).exitCode().run();
}

/** Code block with a language picker and a copy button in its top-right corner, visible on hover or focus. */
export const CodeBlock = CodeBlockLowlight.configure({ lowlight, defaultLanguage: null, enableTabIndentation: true }).extend({
  addKeyboardShortcuts() {
    const inherited = this.parent?.() ?? {};
    return { ...inherited, Enter: (props) => closeFence(props.editor) || (inherited.Enter?.(props) ?? false) };
  },

  addNodeView() {
    return ({ node: initial, editor, getPos }) => {
      let node = initial;
      const dom = document.createElement('div');
      dom.className = 'code-block';

      const tools = document.createElement('div');
      tools.className = 'code-tools';
      tools.contentEditable = 'false';

      const select = document.createElement('select');
      select.className = 'code-lang';
      select.setAttribute('aria-label', 'Code language');
      select.add(new Option('Auto', ''));
      let populated = false;
      const populate = () => {
        if (populated) return;
        populated = true;
        const current = select.value;
        const existing = new Set([...select.options].map((o) => o.value));
        for (const l of COMMON_LANGUAGES) {
          if (!existing.has(l)) select.add(new Option(l, l));
        }
        select.value = current;
      };
      select.addEventListener('pointerdown', populate, { once: true });
      select.addEventListener('focus', populate, { once: true });
      // A fence can name its language by an alias (```js) that the list spells another way; show it as typed.
      const show = (language: string | null) => {
        if (language && ![...select.options].some((o) => o.value === language)) select.add(new Option(language, language), 1);
        select.value = language ?? '';
      };
      show(node.attrs.language as string | null);
      if (!isLowlightReady()) {
        void ensureLowlight().then(() => {
          if (editor.isDestroyed) return;
          const pos = getPos();
          if (pos === undefined) return;
          const cur = editor.state.doc.nodeAt(pos);
          if (cur?.type.name === 'codeBlock') {
            editor.view.dispatch(
              editor.state.tr.setNodeMarkup(pos, undefined, { ...cur.attrs }).setMeta('addToHistory', false).setMeta(QUIET, true),
            );
          }
        });
      }
      select.addEventListener('change', () => {
        const pos = getPos();
        if (pos === undefined) return;
        editor.view.dispatch(
          editor.state.tr.setNodeMarkup(pos, undefined, { ...node.attrs, language: select.value || null }),
        );
      });

      const copy = document.createElement('button');
      copy.type = 'button';
      copy.className = 'code-copy';
      copy.setAttribute('aria-label', 'Copy code');
      copy.title = 'Copy';
      copy.innerHTML = COPY_ICON;
      copy.addEventListener('mousedown', (e) => e.preventDefault());
      copy.addEventListener('click', () => void copyFrom(copy, node.textContent));

      tools.append(select, copy);
      const pre = document.createElement('pre');
      const code = document.createElement('code');
      pre.append(code);
      dom.append(tools, pre);

      return {
        dom,
        contentDOM: code,
        update(next) {
          if (next.type !== node.type) return false;
          node = next;
          show(next.attrs.language as string | null);
          return true;
        },
        stopEvent: (e) => tools.contains(e.target as Node),
        ignoreMutation: (m) => tools.contains(m.target as Node),
      };
    };
  },
});
