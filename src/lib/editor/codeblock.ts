import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight';
import { COPY_ICON, copyFrom } from '../copy';
import { lowlight } from '../rich';

const LANGUAGES = lowlight.listLanguages().sort();

/** Code block with a language picker and a copy button in its top-right corner, visible on hover or focus. */
export const CodeBlock = CodeBlockLowlight.configure({ lowlight, defaultLanguage: null }).extend({
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
          select.value = (next.attrs.language as string | null) ?? '';
          return true;
        },
        stopEvent: (e) => tools.contains(e.target as Node),
        ignoreMutation: (m) => tools.contains(m.target as Node),
      };
    };
  },
});
