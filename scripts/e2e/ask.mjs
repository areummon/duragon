// Write each argument after the app path and output dir as its own line addressed to Duragon, wait for the answers, screenshot.
import { Driver, sleep } from './driver.mjs';

const [app, out, ...lines] = process.argv.slice(2);
const d = await new Driver().start(app);
try {
  await d.waitFor(`document.querySelector('.ProseMirror')`, 30000);
  await sleep(800);
  for (const [i, line] of lines.entries()) {
    const before = await d.exec(`return document.querySelectorAll('[data-inline-answer]').length`);
    await d.exec(`const ed = document.querySelector('.ProseMirror'); ed.focus(); const s = getSelection(); s.selectAllChildren(ed); s.collapseToEnd();`);
    await d.type('.ProseMirror', '' + line);
    await d.waitFor(`document.querySelectorAll('[data-inline-answer]').length > ${before} && !document.body.innerText.includes('Sketching it')`, 240000, 1000);
    await sleep(7000);
    await d.screenshot(`${out}/ask-${i}.png`);
    console.log(line, '=>', await d.exec(`const a = document.querySelectorAll('[data-inline-answer]'); return a[a.length - 1].innerText.slice(0, 400)`));
  }
} finally {
  await d.quit();
}
