// Real end-to-end check of the polish pass: new-note editor, reading pencil, anchor + tether on hover,
// unaddressed request answered inline, left-edge sidebar, New Mind form alone. Makes two real `claude` reads.
import { Driver, sleep } from './driver.mjs';

const app = process.argv[2];
const out = process.argv[3] ?? '.';
const d = await new Driver().start(app);
const shot = (n) => d.screenshot(`${out}/polish-${n}.png`);
const key = (k, ctrl = true) => d.exec(`document.dispatchEvent(new KeyboardEvent('keydown', { key: arguments[0], ctrlKey: arguments[1], bubbles: true }))`, k, ctrl);
const clickText = (text) =>
  d.exec(
    `const b = [...document.querySelectorAll('button')].find((b) => b.textContent.trim().startsWith(arguments[0]));
     if (!b) return false; b.click(); return true;`,
    text,
  );
const check = (label, ok) => console.log(`${ok ? 'PASS' : 'FAIL'} ${label}`);

try {
  await d.waitFor(`document.querySelector('.ProseMirror')`, 30000);
  await sleep(800);

  await key('n');
  await sleep(900);
  check('new note has an editor', await d.exec(`return document.querySelectorAll('.ProseMirror').length === 1`));
  const holders = await d.exec(`return [...document.querySelectorAll('.ProseMirror .is-empty')].map((e) => e.dataset.placeholder)`);
  check(`empty note placeholders ${JSON.stringify(holders)}`, holders.includes('Untitled') && holders.includes('Just start writing.'));
  await shot('1-empty');

  await d.exec(`const ed = document.querySelector('.ProseMirror'); ed.focus(); const s = getSelection(); s.selectAllChildren(ed.lastElementChild); s.collapseToEnd();`);
  await d.type('.ProseMirror', 'The Great Wall of China is easily visible from the Moon with the naked eye.');
  const pencil = await d.waitFor(`document.querySelector('svg.pencil path.stroke')`, 15000, 150);
  check('reading pencil drawn while reading', !!pencil);
  await sleep(700);
  await shot('2-reading');

  const anchor = await d.waitFor(`document.querySelector('.riddle-anchor[data-thread]')`, 120000, 1000);
  check('read produced an anchored thread', !!anchor);
  await sleep(1500);
  await shot('3-anchor');

  await d.exec(`const a = document.querySelector('.riddle-anchor[data-thread]'); a.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));`);
  await sleep(700);
  const hover = await d.exec(`return { tether: !!document.querySelector('svg.thread-tether'), active: document.querySelector('.riddle-anchor[data-thread]').dataset.active }`);
  check(`hover shows tether and activates anchor ${JSON.stringify(hover)}`, hover.tether && hover.active === 'true');
  await shot('4-hover');
  await d.exec(`document.querySelector('.riddle-anchor[data-thread]').dispatchEvent(new MouseEvent('mouseout', { bubbles: true }));`);

  await d.exec(`const ed = document.querySelector('.ProseMirror'); ed.focus(); const s = getSelection(); s.selectAllChildren(ed.lastElementChild); s.collapseToEnd();`);
  await d.type('.ProseMirror', '\uE007Can you explain how a hash map handles collisions');
  const inline = await d.waitFor(`document.querySelector('[data-inline-answer]')?.innerText`, 150000, 1000);
  check(`unaddressed request answered inline: ${JSON.stringify(String(inline).slice(0, 120))}`, !!inline);
  await sleep(2500);
  await shot('5-request');

  await d.exec(`document.querySelector('.edge').dispatchEvent(new MouseEvent('mouseenter'))`);
  await sleep(600);
  check('left edge opens sidebar', await d.exec(`return document.querySelector('nav.sidebar').classList.contains('open')`));
  await shot('6-edge');
  await d.exec(`document.querySelector('nav.sidebar').dispatchEvent(new MouseEvent('mouseleave', { clientX: 900 }))`);
  await sleep(600);
  check('leaving the sidebar closes it', await d.exec(`return !document.querySelector('nav.sidebar').classList.contains('open')`));

  await clickText('Duragon');
  await sleep(400);
  await clickText('New Mind');
  await sleep(700);
  const dialogs = await d.exec(`return [...document.querySelectorAll('[role=dialog]')].map((e) => e.getAttribute('aria-label'))`);
  check(`New Mind opens only its form ${JSON.stringify(dialogs)}`, dialogs.length === 1 && dialogs[0] === 'New Mind');
  await shot('7-new-mind');
} finally {
  await d.quit();
}
