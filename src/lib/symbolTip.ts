import type { MathSymbol } from './api/types';

export interface SymbolTipsParams {
  symbols: MathSymbol[];
  onask: (s: MathSymbol) => void;
}

/** One tooltip for the whole app, since only one symbol is pointed at or focused at a time. */
let tip: HTMLElement | null = null;
let shownFor: HTMLElement | null = null;

function place(anchor: HTMLElement) {
  if (!tip) return;
  const r = anchor.getBoundingClientRect();
  const { width, height } = tip.getBoundingClientRect();
  const left = Math.min(Math.max(8, r.left + r.width / 2 - width / 2), window.innerWidth - width - 8);
  const top = r.top - height - 6 < 8 ? r.bottom + 6 : r.top - height - 6;
  tip.style.left = `${left}px`;
  tip.style.top = `${top}px`;
}

function show(anchor: HTMLElement, text: string) {
  if (!tip) {
    tip = document.createElement('div');
    tip.className = 'symbol-tip';
    tip.setAttribute('role', 'tooltip');
  }
  // A modal dialog sits in the top layer, above anything appended to body.
  const host = anchor.closest('dialog') ?? document.body;
  if (tip.parentElement !== host) host.append(tip);
  tip.textContent = text;
  shownFor = anchor;
  place(anchor);
  tip.classList.add('shown');
  window.addEventListener('scroll', hide, true);
}

function hide() {
  tip?.classList.remove('shown');
  shownFor = null;
  window.removeEventListener('scroll', hide, true);
}

/**
 * Wires the `[data-sym]` spans inside a rendered formula: pointing or focusing shows what the symbol stands for,
 * and a press asks about it. Only a fine pointer that can hover gets the hover path; focus always works.
 */
export function symbolTips(node: HTMLElement, initial: SymbolTipsParams | null) {
  let params = initial;
  const hover = window.matchMedia('(hover: hover) and (pointer: fine)');
  const at = (e: Event) => (e.target as Element | null)?.closest<HTMLElement>('[data-sym]') ?? null;
  const symbolOf = (el: HTMLElement) => params?.symbols[Number(el.dataset.sym)];

  const over = (e: PointerEvent) => {
    const el = at(e);
    const s = el && symbolOf(el);
    if (el && s && hover.matches && shownFor !== el) show(el, s.meaning);
  };
  const out = (e: PointerEvent) => {
    const el = at(e);
    if (el && el === shownFor && !el.contains(e.relatedTarget as Node | null) && document.activeElement !== el) hide();
  };
  const focus = (e: FocusEvent) => {
    const el = at(e);
    const s = el && symbolOf(el);
    if (el && s) show(el, s.meaning);
  };
  const blur = (e: FocusEvent) => {
    if (at(e) === shownFor) hide();
  };
  const ask = (e: Event) => {
    const el = at(e);
    const s = el && symbolOf(el);
    if (!s || !params) return;
    // The formula may sit in something clickable, such as a thread that closes on a press.
    e.stopPropagation();
    e.preventDefault();
    hide();
    params.onask(s);
  };
  const key = (e: KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') ask(e);
    else if (e.key === 'Escape' && at(e) === shownFor && shownFor) {
      e.stopPropagation();
      hide();
    }
  };

  node.addEventListener('pointerover', over);
  node.addEventListener('pointerout', out);
  node.addEventListener('focusin', focus);
  node.addEventListener('focusout', blur);
  node.addEventListener('click', ask);
  node.addEventListener('keydown', key);
  return {
    update(next: SymbolTipsParams | null) {
      params = next;
    },
    destroy() {
      node.removeEventListener('pointerover', over);
      node.removeEventListener('pointerout', out);
      node.removeEventListener('focusin', focus);
      node.removeEventListener('focusout', blur);
      node.removeEventListener('click', ask);
      node.removeEventListener('keydown', key);
      if (shownFor && node.contains(shownFor)) hide();
    },
  };
}

/** Makes each marked symbol in KaTeX HTML a focusable button named by what it stands for. */
export function symbolButtons(html: string, symbols: MathSymbol[]): string {
  const attr = (s: string) => s.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;');
  return html.replace(/data-sym="(\d+)"/g, (m, i: string) => {
    const s = symbols[Number(i)];
    return s ? `${m} tabindex="0" role="button" aria-label="${attr(s.meaning)}"` : m;
  });
}

/** The writer's follow-up when they press a symbol. */
export const askAboutSymbol = (s: MathSymbol) => `Explain $${s.tex}$ in more detail: what does it represent here?`;
