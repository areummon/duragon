import type { ElementContent, Root } from 'hast';
import katex from 'katex';
import { common, createLowlight } from 'lowlight';

export const lowlight = createLowlight(common);

export interface MathRange {
  from: number;
  to: number;
  tex: string;
  display: boolean;
}

export type Inline = { kind: 'text' | 'code' | 'math'; text: string };
export type Block =
  | { kind: 'para'; inlines: Inline[] }
  | { kind: 'code'; lang: string; code: string }
  | { kind: 'math'; tex: string };

const isSpace = (c: string | undefined) => c !== undefined && /\s/.test(c);
const isDigit = (c: string | undefined) => c !== undefined && c >= '0' && c <= '9';

/** Index of the next `close` at or after `from` that is not escaped with a backslash, or -1. */
function closing(text: string, close: string, from: number, stopAtNewline: boolean): number {
  for (let i = from; i < text.length; i++) {
    const c = text[i];
    if (c === '\\') {
      if (text.startsWith(close, i)) return i;
      i++;
      continue;
    }
    if (stopAtNewline && c === '\n') return -1;
    if (text.startsWith(close, i)) return i;
  }
  return -1;
}

/**
 * Math in one block of text, pandoc style: `$$…$$` and `\[…\]` display, `$…$` and `\(…\)` inline.
 * An inline `$` opens only before a non-space and closes only after a non-space and before a non-digit,
 * so prices like "$5 and $10" stay text. `\$` is a literal dollar.
 */
export function findMath(text: string): MathRange[] {
  const out: MathRange[] = [];
  let i = 0;
  while (i < text.length) {
    const c = text[i];
    if (c === '\\') {
      const n = text[i + 1];
      if (n === '(' || n === '[') {
        const close = n === '(' ? '\\)' : '\\]';
        const end = closing(text, close, i + 2, n === '(');
        const tex = end < 0 ? '' : text.slice(i + 2, end);
        if (tex.trim()) {
          out.push({ from: i, to: end + 2, tex: tex.trim(), display: n === '[' });
          i = end + 2;
          continue;
        }
      }
      i += 2;
      continue;
    }
    if (c !== '$') {
      i++;
      continue;
    }
    if (text[i + 1] === '$') {
      const end = closing(text, '$$', i + 2, false);
      const tex = end < 0 ? '' : text.slice(i + 2, end);
      if (tex.trim()) {
        out.push({ from: i, to: end + 2, tex: tex.trim(), display: true });
        i = end + 2;
      } else {
        i += 2;
      }
      continue;
    }
    const end = closing(text, '$', i + 1, true);
    if (end > i + 1 && !isSpace(text[i + 1]) && !isSpace(text[end - 1]) && !isDigit(text[end + 1])) {
      out.push({ from: i, to: end + 1, tex: text.slice(i + 1, end), display: false });
      i = end + 1;
    } else {
      i++;
    }
  }
  return out;
}

const FENCE = /^\s*```\s*([\w+#.-]*)\s*$/;
const CLOSE_FENCE = /^\s*```\s*$/;

/** A model reply as blocks: fenced code, displayed math and paragraphs of text, inline code and inline math. */
export function parseReply(text: string): Block[] {
  const blocks: Block[] = [];
  let para: string[] = [];
  const flush = () => {
    if (para.length) blocks.push(...paragraph(para.join('\n')));
    para = [];
  };
  const lines = text.split('\n');
  for (let i = 0; i < lines.length; i++) {
    const fence = FENCE.exec(lines[i]);
    if (fence) {
      flush();
      const code: string[] = [];
      while (++i < lines.length && !CLOSE_FENCE.test(lines[i])) code.push(lines[i]);
      blocks.push({ kind: 'code', lang: fence[1].toLowerCase(), code: code.join('\n') });
    } else if (!lines[i].trim()) {
      flush();
    } else {
      para.push(lines[i]);
    }
  }
  flush();
  return blocks;
}

/** One paragraph; a displayed formula inside it splits it, since display math always stands on its own line. */
function paragraph(text: string): Block[] {
  const blocks: Block[] = [];
  let inlines: Inline[] = [];
  const push = (kind: Inline['kind'], s: string) => {
    if (kind === 'text') s = s.replace(/\\\$/g, '$');
    if (s) inlines.push({ kind, text: s });
  };
  const end = () => {
    const first = inlines[0];
    const last = inlines[inlines.length - 1];
    if (first?.kind === 'text') first.text = first.text.trimStart();
    if (last?.kind === 'text') last.text = last.text.trimEnd();
    inlines = inlines.filter((x) => x.text);
    if (inlines.length) blocks.push({ kind: 'para', inlines });
    inlines = [];
  };
  for (const part of text.trim().split(/(`[^`\n]+`)/)) {
    if (part.length > 2 && part.startsWith('`') && part.endsWith('`')) {
      push('code', part.slice(1, -1));
      continue;
    }
    let at = 0;
    for (const m of findMath(part)) {
      push('text', part.slice(at, m.from));
      if (m.display) {
        end();
        blocks.push({ kind: 'math', tex: m.tex });
      } else {
        push('math', m.tex);
      }
      at = m.to;
    }
    push('text', part.slice(at));
  }
  end();
  return blocks;
}

const texCache = new Map<string, string>();

/** KaTeX HTML for a formula. Bad LaTeX renders as its red source instead of throwing. */
export function renderTex(tex: string, display: boolean): string {
  const key = (display ? 'D' : 'I') + tex;
  let html = texCache.get(key);
  if (html === undefined) {
    html = katex.renderToString(tex, { displayMode: display, throwOnError: false });
    texCache.set(key, html);
  }
  return html;
}

const escape = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

function hastHtml(nodes: ElementContent[]): string {
  return nodes
    .map((n) => {
      if (n.type === 'text') return escape(n.value);
      if (n.type !== 'element') return '';
      const cls = n.properties?.className;
      const attr = Array.isArray(cls) && cls.length ? ` class="${escape(cls.join(' '))}"` : '';
      return `<span${attr}>${hastHtml(n.children)}</span>`;
    })
    .join('');
}

/** Code as HTML with `hljs-*` token spans; unknown or missing languages are auto-detected. */
export function highlight(code: string, lang: string): string {
  const tree: Root = lang && lowlight.registered(lang) ? lowlight.highlight(lang, code) : lowlight.highlightAuto(code);
  return hastHtml(tree.children as ElementContent[]);
}

/** TeX source as tokens: a control word with its trailing spaces, a control symbol, a brace, a space run or one character. */
function texTokens(tex: string): string[] {
  return tex.match(/\\[a-zA-Z]+\s*|\\.|\s+|[^]/gu) ?? [];
}

const bare = (t: string) => t.trimEnd();

/** Commands whose braced argument is a name or words, never symbols. */
const OPAQUE = new Set(['\\text', '\\textrm', '\\textit', '\\textbf', '\\textsf', '\\texttt', '\\mbox', '\\hbox',
  '\\mathrm', '\\operatorname', '\\begin', '\\end', '\\color', '\\textcolor', '\\label', '\\htmlData']);

/** Index after the braced group that opens at `i` (or after the single token there). */
function skipGroup(toks: string[], i: number): number {
  while (i < toks.length && (!toks[i].trim() || toks[i] === '*')) i++;
  if (toks[i] !== '{') return i + 1;
  for (let depth = 0; i < toks.length; i++) {
    if (toks[i] === '{') depth++;
    else if (toks[i] === '}' && --depth === 0) return i + 1;
  }
  return i;
}

/** A symbol's tokens without spaces or enclosing braces, with `{x}` around a single token read as `x`. */
function symbolPattern(tex: string): string[] {
  let t = texTokens(tex).map(bare).filter((x) => x);
  while (t[0] === '{' && closes(t) === t.length - 1) t = t.slice(1, -1);
  const out: string[] = [];
  for (let i = 0; i < t.length; i++) {
    if (t[i] === '{' && t[i + 2] === '}' && t[i + 1] !== '{' && t[i + 1] !== '}') {
      out.push(t[i + 1]);
      i += 2;
    } else out.push(t[i]);
  }
  return out;
}

/** Index of the brace that closes the one at 0. */
function closes(t: string[]): number {
  let depth = 0;
  for (let i = 0; i < t.length; i++) {
    if (t[i] === '{') depth++;
    else if (t[i] === '}' && --depth === 0) return i;
  }
  return -1;
}

/** Where `pat` ends if it matches the formula at `i`, or -1. Spaces inside the match and `{x}` for `x` are allowed. */
function matchAt(toks: string[], i: number, pat: string[]): number {
  for (const p of pat) {
    while (i < toks.length && !toks[i].trim()) i++;
    if (bare(toks[i] ?? '') === p) i++;
    else if (toks[i] === '{' && bare(toks[i + 1] ?? '') === p && toks[i + 2] === '}') i += 3;
    else return -1;
  }
  return i;
}

/**
 * The formula with each occurrence of a listed symbol wrapped as `{\htmlData{sym=<index>}{…}}`, matching whole tokens
 * so `m` never matches inside `\mathrm` and `x` never inside `\exp`, longest symbol first. The outer braces keep the
 * wrapped symbol a single argument, as in `x^n` or `\frac m v`.
 */
export function markSymbols(tex: string, symbols: string[]): string {
  const pats = symbols
    .map((s, index) => ({ index, pat: symbolPattern(s) }))
    .filter((p) => p.pat.length && !p.pat.every((t) => t === '{' || t === '}'))
    .sort((a, b) => b.pat.length - a.pat.length);
  const toks = texTokens(tex);
  let out = '';
  for (let i = 0; i < toks.length; ) {
    if (toks[i].trim()) {
      const hit = pats.map((p) => ({ ...p, end: matchAt(toks, i, p.pat) })).find((p) => p.end > i);
      if (hit) {
        const body = toks.slice(i, hit.end).join('');
        const space = body.length - body.trimEnd().length;
        out += `{\\htmlData{sym=${hit.index}}{${body.trimEnd()}}}${body.slice(body.length - space)}`;
        i = hit.end;
        continue;
      }
      if (OPAQUE.has(bare(toks[i]))) {
        const end = skipGroup(toks, i + 1);
        out += toks.slice(i, end).join('');
        i = end;
        continue;
      }
    }
    out += toks[i++];
  }
  return out;
}

const markedCache = new Map<string, string>();

/**
 * KaTeX HTML for a formula whose listed symbols carry `data-sym="<index>"`. HTML output only, and not hidden from
 * assistive tech, since the symbols inside become buttons. Falls back to `renderTex` when the marked source fails.
 */
export function renderTexWithSymbols(tex: string, display: boolean, symbols: string[]): string {
  const key = JSON.stringify([display, tex, symbols]);
  let html = markedCache.get(key);
  if (html === undefined) {
    const marked = markSymbols(tex, symbols);
    try {
      html = marked === tex
        ? renderTex(tex, display)
        : katex
            .renderToString(marked, {
              displayMode: display,
              throwOnError: true,
              output: 'html',
              trust: (c) => c.command === '\\htmlData',
              strict: (code: string) => (code === 'htmlExtension' ? 'ignore' : 'warn'),
            })
            .replace('<span class="katex-html" aria-hidden="true">', '<span class="katex-html">');
    } catch {
      html = renderTex(tex, display);
    }
    markedCache.set(key, html);
  }
  return html;
}
