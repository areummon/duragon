const DURATION = 220;

/** Keeps a block in view while it unfolds. */
export function follow(node: HTMLElement) {
  const end = performance.now() + DURATION;
  const step = () => {
    node.scrollIntoView({ block: 'nearest' });
    if (node.isConnected && performance.now() < end) requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
}
