/** Lucide's copy and check icons as markup, so the editor's plain-DOM code blocks and replies show the same button. */
const svg = (body: string) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;
export const COPY_ICON = svg('<rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>');
export const COPIED_ICON = svg('<path d="M20 6 9 17l-5-5"/>');
const COPIED_MS = 1200;

/** Copies `text` and shows a check on `button` for a moment. */
export async function copyFrom(button: HTMLElement, text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    return;
  }
  button.innerHTML = COPIED_ICON;
  button.setAttribute('aria-label', 'Copied');
  button.classList.add('copied');
  window.setTimeout(() => {
    button.innerHTML = COPY_ICON;
    button.setAttribute('aria-label', 'Copy code');
    button.classList.remove('copied');
  }, COPIED_MS);
}
