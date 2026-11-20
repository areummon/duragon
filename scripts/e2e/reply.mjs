// Real end-to-end check of writing back to an inline answer: it unfolds in place, a reply is answered in the
// thread, folding shows the reply count, the hover menu offers Move to margin, and the move survives a restart.
// Makes two real `claude` calls (one read, one reply).
import { Driver, sleep } from './driver.mjs';

const app = process.argv[2];
const out = process.argv[3] ?? '.';
let d = await new Driver().start(app);
const shot = (n) => d.screenshot(`${out}/reply-${n}.png`);
const key = (k) => d.exec(`document.dispatchEvent(new KeyboardEvent('keydown', { key: arguments[0], ctrlKey: true, bubbles: true }))`, k);
const check = (label, ok) => console.log(`${ok ? 'PASS' : 'FAIL'} ${label}`);
const inline = `document.querySelector('[data-inline-answer]')`;

try {
  await d.waitFor(`document.querySelector('.ProseMirror')`, 30000);
  await sleep(800);
  await key('n');
  await sleep(900);

  await d.exec(`const ed = document.querySelector('.ProseMirror'); ed.focus(); const s = getSelection(); s.selectAllChildren(ed.lastElementChild); s.collapseToEnd();`);
  await d.type('.ProseMirror', 'Can you explain what a bloom filter is in two sentences?');
  const answered = await d.waitFor(`${inline}?.querySelector('.text')?.innerText`, 150000, 1000);
  check(`request answered inline: ${JSON.stringify(String(answered).slice(0, 80))}`, !!answered);
  await sleep(4000);

  await d.exec(`${inline}.querySelector('.text').click()`);
  await sleep(500);
  const opened = await d.exec(`
    const t = document.querySelector('[aria-label="Conversation with Duragon"]');
    return t && {
      focused: document.activeElement === t.querySelector('textarea'),
      placeholder: t.querySelector('textarea').placeholder,
      controls: [...t.querySelectorAll('.controls button')].map((b) => b.getAttribute('aria-label') || b.textContent.trim()),
      animation: getComputedStyle(t).animationName,
      slots: document.querySelectorAll('.slot').length,
    }`);
  check(`clicking the answer unfolds it in place ${JSON.stringify(opened)}`,
    !!opened && opened.focused && opened.placeholder === 'Write back…' && opened.animation === 'unfold'
    && JSON.stringify(opened.controls) === JSON.stringify(['Dismiss', 'Pin open while you write', 'Open full screen', 'Thread options']));
  await shot('1-open');

  await d.type('[aria-label="Conversation with Duragon"] textarea', 'And what decides its false positive rate?');
  await sleep(400);
  const pending = await d.exec(`
    const t = document.querySelector('[aria-label="Conversation with Duragon"]');
    return { you: [...t.querySelectorAll('.msg.you')].map((m) => m.innerText), waiting: !!t.querySelector('.thinking'), editor: document.querySelector('.ProseMirror').childElementCount }`);
  check(`reply shows at once with a waiting blot ${JSON.stringify(pending)}`, pending.you.length === 1 && pending.waiting);
  await shot('2-waiting');
  const reply = await d.waitFor(`document.querySelectorAll('[aria-label="Conversation with Duragon"] .msg.riddled')[1]?.innerText`, 150000, 1000);
  check(`Duragon answers in the thread: ${JSON.stringify(String(reply).slice(0, 80))}`, !!reply);
  await sleep(2500);
  await shot('3-answered');

  await d.exec(`document.querySelector('.page-title, h1, body').dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))`);
  await sleep(500);
  const folded = await d.exec(`return { open: !!document.querySelector('[aria-label="Conversation with Duragon"]'), replies: ${inline}.querySelector('.replies')?.innerText }`);
  check(`a press outside folds it with the reply count ${JSON.stringify(folded)}`, !folded.open && folded.replies === '2 replies');

  await d.exec(`${inline}.querySelector('[aria-label="Thread options"]').click()`);
  await sleep(300);
  const items = await d.exec(`return [...document.querySelectorAll('[role=menuitem]')].map((b) => b.textContent)`);
  check(`hover menu ${JSON.stringify(items)}`, JSON.stringify(items) === JSON.stringify(['Copy', 'Read aloud', 'Make a note', 'Move to margin', 'Dismiss']));
  await shot('4-menu');
  await d.exec(`[...document.querySelectorAll('[role=menuitem]')].find((b) => b.textContent === 'Move to margin').click()`);
  await sleep(1200);
  const moved = await d.exec(`return { inline: document.querySelectorAll('[data-inline-answer]').length, margin: document.querySelectorAll('.slot article').length, kind: document.querySelector('.riddle-anchor')?.dataset.kind }`);
  check(`Move to margin ${JSON.stringify(moved)}`, moved.inline === 0 && moved.margin === 1 && moved.kind === 'question');
  await shot('5-moved');

  await d.quit();
  d = await new Driver().start(app);
  await d.waitFor(`document.querySelector(".ProseMirror")`, 30000);
  await sleep(2500);
  const kept = await d.exec(`return { inline: document.querySelectorAll('[data-inline-answer]').length, margin: document.querySelectorAll('.slot article').length }`);
  check(`the move is saved ${JSON.stringify(kept)}`, kept.inline === 0 && kept.margin === 1);
} finally {
  await d.quit();
}
