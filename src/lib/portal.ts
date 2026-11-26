/** The page stage slides with a transform when chat or the sidebar opens, which would re-anchor a fixed menu. */
export function portal(node: HTMLElement) {
  document.body.appendChild(node);
  return { destroy: () => node.remove() };
}
