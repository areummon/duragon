// Real end-to-end check: real Rust backend, real `claude` CLI, real window.
// Writes a false sentence, waits for a margin question, then asks for an inline drawing.
import { Driver, sleep } from './driver.mjs';

const app = process.argv[2];
const out = process.argv[3] ?? '.';
const d = await new Driver().start(app);
const shot = (n) => d.screenshot(`${out}/e2e-${n}.png`);

try {
  await d.waitFor(`document.querySelector('.ProseMirror')`, 30000);
  await sleep(800);
  await shot('0-open');

  await d.exec(`
    const ed = document.querySelector('.ProseMirror');
    ed.focus();
    const sel = getSelection(); sel.selectAllChildren(ed); sel.collapseToEnd();`);
  await d.type('.ProseMirror', 'The heart pumps blood only to the lungs, never to the rest of the body.');

  const q = await d.waitFor(`document.querySelector('[data-thread-id], .thread, article')?.innerText`, 90000);
  await sleep(2500);
  console.log('margin question:', JSON.stringify(q).slice(0, 300));
  await shot('1-question');

  await d.type('.ProseMirror', 'Duragon, draw how blood flows through the heart.');
  await d.waitFor(`[...document.querySelectorAll('svg [data-step]')].length > 0`, 180000, 1000);
  await sleep(6000);
  await shot('2-drawing');
  console.log('drawing steps:', await d.exec(`return document.querySelectorAll('svg [data-step]').length`));
} finally {
  await d.quit();
}
