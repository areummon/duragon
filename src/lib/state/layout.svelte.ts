import type { Range } from '../editor/quote';
import { PAGE_WIDTH_DEFAULT, settings } from './settings.svelte';

export const MARGIN_GAP = 40;
export const MARGIN_MAX = 360;
const MIN_COLUMN = 220;

export const layout = $state({ vw: window.innerWidth, scrolled: false, resizing: false });
export const selectionUi = $state<{ pin: Range | null }>({ pin: null });

window.addEventListener('resize', () => (layout.vw = window.innerWidth));
window.addEventListener('scroll', () => (layout.scrolled = window.scrollY > 40), { passive: true });

/** Width the margin columns can take, or 0 when the window is too narrow. */
export function marginWidth(): number {
  const page = Math.min(settings.value.pageWidth ?? PAGE_WIDTH_DEFAULT, layout.vw - 48);
  const free = (layout.vw - page) / 2 - MARGIN_GAP - 24;
  return free < MIN_COLUMN ? 0 : Math.min(MARGIN_MAX, free);
}

/** Questions show as "?" marks when asked to, or when there is no room for margins. */
export const useMarks = () => settings.value.questionPlacement === 'marks' || marginWidth() === 0;

export const lightbox = $state<{ svg: string | null }>({ svg: null });
export const enlarge = (svg: string | null) => (lightbox.svg = svg);
