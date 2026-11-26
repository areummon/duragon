import type { Editor, Range } from '@tiptap/core';
import type { ResolvedPos } from '@tiptap/pm/model';
import { Plugin, PluginKey } from '@tiptap/pm/state';

export interface InlineMenuState {
  open: boolean;
  query: string;
  index: number;
  from: number;
  to: number;
  x: number;
  y: number;
}

export interface InlineMenuConfig<T> {
  key: string;
  width: number;
  trigger: RegExp;
  triggerLength: number;
  canOpen: (head: ResolvedPos) => boolean;
  matches: (query: string) => T[];
  select: (editor: Editor, item: T, range: Range) => void;
}

export interface InlineMenu<T> {
  readonly state: InlineMenuState;
  readonly width: number;
  matches: (query?: string) => T[];
  dismiss: () => void;
  choose: (editor: Editor, item: T) => void;
  createPlugin: (editor: Editor) => Plugin;
}

const registeredMenus: InlineMenuState[] = [];

export function anyInlineMenuOpen(): boolean {
  return registeredMenus.some((m) => m.open);
}

export function createInlineMenu<T>(config: InlineMenuConfig<T>): InlineMenu<T> {
  const state = $state<InlineMenuState>({
    open: false,
    query: '',
    index: 0,
    from: 0,
    to: 0,
    x: 0,
    y: 0,
  });
  registeredMenus.push(state);

  const pluginKey = new PluginKey(config.key);
  let dismissedAt = -1;

  function close() {
    state.open = false;
    state.index = 0;
  }

  /** Escape: closes the menu and keeps it closed while the caret stays after this trigger. */
  function dismiss() {
    dismissedAt = state.from;
    close();
  }

  function matches(query = state.query): T[] {
    return config.matches(query);
  }

  function choose(editor: Editor, item: T) {
    const range: Range = { from: state.from, to: state.to };
    close();
    config.select(editor, item, range);
  }

  function createPlugin(editor: Editor): Plugin {
    return new Plugin({
      key: pluginKey,
      view: () => ({
        update(view) {
          const { selection } = view.state;
          const { $from: head } = selection;
          if (!selection.empty || !config.canOpen(head)) return close();
          const before = head.parent.textBetween(0, head.parentOffset, undefined, '\ufffc');
          const m = config.trigger.exec(before);
          if (!m) {
            dismissedAt = -1;
            return close();
          }
          const query = m[1];
          const from = selection.from - query.length - config.triggerLength;
          if (dismissedAt === from) return close();
          if (config.matches(query).length === 0) return close();
          const coords = view.coordsAtPos(from);
          if (state.query !== query || !state.open) state.index = 0;
          const x = Math.max(12, Math.min(coords.left, window.innerWidth - (config.width + 12)));
          state.open = true;
          state.query = query;
          state.from = from;
          state.to = selection.from;
          state.x = x;
          state.y = coords.bottom + 8;
        },
        destroy: close,
      }),
      props: {
        // Escape is owned exclusively by InlineMenuPopup's closeOnEscape layer so it never races App.svelte's closeTop().
        handleKeyDown(_view, event) {
          if (!state.open) return false;
          const items = matches();
          if (items.length === 0) return false;
          if (event.key === 'ArrowDown') state.index = (state.index + 1) % items.length;
          else if (event.key === 'ArrowUp') state.index = (state.index - 1 + items.length) % items.length;
          else if (event.key === 'Enter' || event.key === 'Tab') choose(editor, items[state.index] ?? items[0]);
          else return false;
          event.preventDefault();
          return true;
        },
      },
    });
  }

  return {
    state,
    width: config.width,
    matches,
    dismiss,
    choose,
    createPlugin,
  };
}
