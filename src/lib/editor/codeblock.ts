import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight';
import { lowlight } from '../rich';

const LANGUAGES = lowlight.listLanguages().sort();

/** Code block with a language picker in its top-right corner, visible on hover or focus. */
export const CodeBlock = CodeBlockLowlight.configure({ lowlight, defaultLanguage: null }).extend({
  addNodeView() {
    return ({ node: initial, editor, getPos }) => {
      let node = initial;
      const dom = document.createElement('div');
      dom.className = 'code-block';

      const select = document.createElement('select');
      select.className = 'code-lang';
      select.contentEditable = 'false';
      select.setAttribute('aria-label', 'Code language');
      const options: [string, string][] = [['', 'Auto'], ...LANGUAGES.map((l): [string, string] => [l, l])];
      for (const [value, label] of options) select.add(new Option(label, value));
      select.value = (node.attrs.language as string | null) ?? '';
      select.addEventListener('change', () => {
        const pos = getPos();
        if (pos === undefined) return;
        editor.view.dispatch(
          editor.state.tr.setNodeMarkup(pos, undefined, { ...node.attrs, language: select.value || null }),
        );
      });

      const pre = document.createElement('pre');
      const code = document.createElement('code');
      pre.append(code);
      dom.append(select, pre);

      return {
        dom,
        contentDOM: code,
        update(next) {
          if (next.type !== node.type) return false;
          node = next;
          select.value = (next.attrs.language as string | null) ?? '';
          return true;
        },
        stopEvent: (e) => e.target === select,
        ignoreMutation: (m) => m.target === select || select.contains(m.target as Node),
      };
    };
  },
});
