// Real end-to-end tour: voice commands, chat panel, web answer with sources, Customize.
import { Driver, sleep } from './driver.mjs';

const app = process.argv[2];
const out = process.argv[3] ?? '.';
const d = await new Driver().start(app);
const shot = (n) => d.screenshot(`${out}/full-${n}.png`);
const clickText = (text) =>
  d.exec(
    `const b = [...document.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(arguments[0]) || b.getAttribute('aria-label') === arguments[0]);
     if (!b) return false; b.click(); return true;`,
    text,
  );
const invoke = (cmd, args = {}) =>
  d.execAsync(`window.__TAURI_INTERNALS__.invoke(args[0], args[1]).then(done, (e) => done({ error: String(e) }));`, cmd, args);

try {
  await d.waitFor(`document.querySelector('.ProseMirror')`, 30000);
  await sleep(800);

  console.log('voice_status:', JSON.stringify(await invoke('voice_status')));
  console.log('list_voices:', JSON.stringify(await invoke('list_voices')));

  console.log('web switch:', await clickText('Web'));
  await d.exec(`const ed = document.querySelector('.ProseMirror'); ed.focus(); const s = getSelection(); s.selectAllChildren(ed); s.collapseToEnd();`);
  await d.type('.ProseMirror', 'Duragon, what is the latest stable Rust version?');
  const src = await d.waitFor(`document.querySelector('.sources')?.innerText`, 120000, 1000);
  console.log('sources line:', JSON.stringify(src));
  await sleep(2000);
  await shot('1-web');

  await d.exec(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'j', ctrlKey: true, bubbles: true }))`);
  await sleep(800);
  console.log('chip:', await clickText('What should I work on next?'));
  const n = await d.waitFor(`document.querySelectorAll('aside.panel .riddled').length`, 120000, 1000);
  await sleep(4000);
  await shot('2-chat');
  console.log('chat answers:', n);
  await d.exec(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'j', ctrlKey: true, bubbles: true }))`);
  await sleep(600);

  await d.exec(`document.dispatchEvent(new KeyboardEvent('keydown', { key: ',', ctrlKey: true, bubbles: true }))`);
  await sleep(800);
  await clickText('Voice models');
  await sleep(600);
  await shot('3-voice-models');
} finally {
  await d.quit();
}
