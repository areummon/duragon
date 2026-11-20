import type { Node as PMNode } from '@tiptap/pm/model';
import type { ThreadStatus } from '../api/types';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { blotSvg } from '../ui/blot';
import { findQuote, type Range } from './quote';

export interface AnchorThread {
  id: string;
  quote: string;
  kind: 'question' | 'reply';
  status: ThreadStatus;
}

export interface AnchorInput {
  threads: AnchorThread[];
  openId: string | null;
  hoverId: string | null;
  /** Threads that just arrived: their underline fades in under the fading pencil. */
  fresh: string[];
  /** Thoughts render as a blot after the quote and open as an inline card. */
  marks: boolean;
  hidden: boolean;
  /** Keeps the selection visible while the ask input has focus. */
  pin: Range | null;
}

export interface AnchorHandlers {
  onOpen(id: string): void;
  onHover(id: string | null): void;
  mountReply(id: string): { dom: HTMLElement; destroy(): void };
  mountCard(id: string): { dom: HTMLElement; destroy(): void };
}

interface AnchorState {
  input: AnchorInput;
  deco: DecorationSet;
  ranges: Map<string, Range | null>;
}

export const anchorKey = new PluginKey<AnchorState>('duragon-anchors');

export const EMPTY_INPUT: AnchorInput = {
  threads: [], openId: null, hoverId: null, fresh: [], marks: false, hidden: false, pin: null,
};

const blockEnd = (doc: PMNode, pos: number) => doc.resolve(pos).after(1);

function anchorAttrs(t: AnchorThread, input: AnchorInput) {
  const active = t.id === input.openId || t.id === input.hoverId;
  const attrs: Record<string, string> = {
    class: 'riddle-anchor',
    'data-thread': t.id,
    'data-state': t.status,
    'data-active': String(active),
    'data-kind': t.kind,
  };
  if (input.fresh.includes(t.id)) attrs['data-fresh'] = 'true';
  return attrs;
}

function build(doc: PMNode, input: AnchorInput, h: AnchorHandlers): AnchorState {
  const ranges = new Map<string, Range | null>();
  const decos: Decoration[] = [];
  for (const t of input.threads) ranges.set(t.id, findQuote(doc, t.quote));
  if (input.hidden) return { input, deco: DecorationSet.empty, ranges };

  for (const t of input.threads) {
    const r = ranges.get(t.id);
    if (!r) continue;
    decos.push(Decoration.inline(r.from, r.to, anchorAttrs(t, input)));
    if (input.marks) {
      decos.push(
        Decoration.widget(r.to, () => {
          const b = document.createElement('button');
          b.className = 'q-mark';
          b.type = 'button';
          b.innerHTML = blotSvg(t.id, 13) + '<i></i>';
          b.setAttribute('aria-label', 'Open Riddled’s thought');
          b.addEventListener('mousedown', (e) => e.preventDefault());
          b.addEventListener('click', () => h.onOpen(t.id));
          return b;
        }, { side: 1, key: `mark-${t.id}`, ignoreSelection: true }),
      );
      if (t.id === input.openId) {
        decos.push(
          Decoration.widget(blockEnd(doc, r.to - 1), () => {
            const m = h.mountCard(t.id);
            cleanups.set(m.dom, m.destroy);
            return m.dom;
          }, { side: 1, key: `card-${t.id}`, ignoreSelection: true, destroy: (n) => cleanups.get(n as HTMLElement)?.() }),
        );
      }
    } else if (t.kind === 'reply') {
      decos.push(
        Decoration.widget(blockEnd(doc, r.to - 1), () => {
          const m = h.mountReply(t.id);
          cleanups.set(m.dom, m.destroy);
          return m.dom;
        }, { side: 1, key: `reply-${t.id}`, ignoreSelection: true, destroy: (n) => cleanups.get(n as HTMLElement)?.() }),
      );
    }
  }

  if (input.pin && input.pin.to <= doc.content.size) {
    decos.push(Decoration.inline(input.pin.from, input.pin.to, { class: 'sel-pin' }));
  }
  return { input, deco: DecorationSet.create(doc, decos), ranges };
}

const cleanups = new WeakMap<HTMLElement, () => void>();

export function anchorPlugin(h: AnchorHandlers) {
  return new Plugin<AnchorState>({
    key: anchorKey,
    state: {
      init: (_, state) => build(state.doc, EMPTY_INPUT, h),
      apply(tr, prev, _old, state) {
        const next = tr.getMeta(anchorKey) as AnchorInput | undefined;
        if (!next && !tr.docChanged) return prev;
        return build(state.doc, next ?? prev.input, h);
      },
    },
    props: {
      decorations: (state) => anchorKey.getState(state)?.deco ?? DecorationSet.empty,
      handleDOMEvents: {
        click(_view, e) {
          const el = (e.target as HTMLElement).closest<HTMLElement>('[data-thread]');
          if (!el) return false;
          h.onOpen(el.dataset.thread!);
          return false;
        },
        mouseover(_view, e) {
          const el = (e.target as HTMLElement).closest<HTMLElement>('[data-thread]');
          if (el) h.onHover(el.dataset.thread!);
          return false;
        },
        mouseout(_view, e) {
          if ((e.target as HTMLElement).closest('[data-thread]')) h.onHover(null);
          return false;
        },
      },
    },
  });
}
