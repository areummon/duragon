import {
  BUILTIN_RIDDLER_ID,
  type MathSymbol,
  type Note,
  type NoteRevision,
  type Thread,
  type VoiceOption,
} from '../types';

export interface StoredRevision extends NoteRevision {
  doc: unknown;
}

interface DocNode {
  type?: unknown;
  text?: unknown;
  attrs?: Record<string, unknown>;
  content?: unknown[];
}

function isDocNode(value: unknown): value is DocNode {
  return typeof value === 'object' && value !== null;
}

export const uid = () => crypto.randomUUID();
export const now = () => Date.now();
export const para = (text: string) => ({ type: 'paragraph', content: [{ type: 'text', text }] });
export const title = (text: string) => ({ type: 'title', content: [{ type: 'text', text }] });

export function maskKey(stored: string | undefined): string | null {
  if (!stored) return null;
  return stored.length <= 8 ? '••••' : `${stored.slice(0, 4)}…${stored.slice(-4)}`;
}

export function plainOf(doc: unknown): string {
  const out: string[] = [];
  const walk = (node: unknown) => {
    if (!isDocNode(node)) return;
    if (node.type === 'text' && typeof node.text === 'string') out.push(node.text);
    else if (node.type === 'noteLink' && typeof node.attrs?.title === 'string') out.push(node.attrs.title);
    if (Array.isArray(node.content)) node.content.forEach(walk);
    if (node.type !== 'doc' && node.type !== 'text' && node.type !== 'noteLink') out.push('\n');
  };
  walk(doc);
  return out.join('').trim();
}

export function hasNoteLink(doc: unknown, targetId: string): boolean {
  let found = false;
  const walk = (node: unknown) => {
    if (found || !isDocNode(node)) return;
    if (node.type === 'noteLink' && node.attrs?.id === targetId) {
      found = true;
      return;
    }
    if (Array.isArray(node.content)) node.content.forEach(walk);
  };
  walk(doc);
  return found;
}

export const QUOTE = 'Photosynthesis happens in the mitochondria.';
export const hash = (s: string) => [...s].reduce((h, c) => (Math.imul(h, 31) + c.charCodeAt(0)) >>> 0, 7);
export const DRAW = /draw|sketch|diagram|picture|visuali[sz]e|show me/i;
export const REQUEST = /^(?:draw|sketch|explain|show|tell|describe|compare|list|summari[sz]e|visuali[sz]e|what|why|how|who|when|where|can|could|should|is|are|do|does)\b/i;
export const QUESTIONS = [
  'What makes you sure about that? Is there a case where it would not hold?',
  'Who is this for, and what would they need to already know?',
  'You say this as if it were settled. What would count as evidence against it?',
  'What happens at the edges here, when the input is empty or enormous?',
];
export const CHAT_ANSWER = `The part I would look at again is the claim that the whole process happens in one place. Reading the note as a whole, it treats two separate jobs as one.\n\nThe first job is capturing something from outside. The second is turning it into something the rest of the system can use. They happen in different places, and the note only names one of them.\n\nA good next step is to write one sentence for each job, then see whether the note still reads the same.`;
/** Asked about a formula or code, the mock answers with every kind of rich content a reply can carry. */
export const RICH = /formula|code/i;
export const RICH_ANSWER = [
  'The area grows with the square of the radius, $A = \\pi r^2$, so doubling $r$ quadruples it. Summed over thin rings it is',
  '$$A = \\int_0^r 2\\pi s\\,ds = \\pi r^2$$',
  'In code, `area(r)` is one line:',
  '```python\nimport math\n\ndef area(r: float) -> float:\n    return math.pi * r ** 2  # square of the radius\n```',
  'What happens to $A$ when $r$ is negative?',
].join('\n\n');
export const RICH_SYMBOLS: MathSymbol[] = [
  { tex: 'A', meaning: 'area of the circle' },
  { tex: '\\pi', meaning: 'circumference over diameter, about 3.14' },
  { tex: 'r', meaning: 'radius of the circle' },
  { tex: 's', meaning: 'radius of one thin ring' },
];

export const SOURCES = [
  { url: 'https://en.wikipedia.org/wiki/Photosynthesis', title: 'Photosynthesis, Wikipedia' },
  { url: 'https://www.khanacademy.org/science/biology', title: 'Biology, Khan Academy' },
];

export function sampleDrawing(): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 170" style="fill:none;stroke-linecap:round;stroke-linejoin:round">
<g data-step="1" data-caption="Light comes in">
<path d="M20 30 Q40 52 62 74" stroke="var(--ink)" stroke-width="1.3" marker-end="url(#arrow)"/>
<path d="M30 20 Q52 44 74 66" stroke="var(--ink)" stroke-width="1.6"/>
<text x="10" y="14" font-size="12" fill="var(--graphite)" style="font-family:var(--font-sans)">sunlight</text>
</g>
<g data-step="2" data-caption="The chloroplast catches it">
<path d="M110 100 Q112 70 160 68 Q208 70 210 100 Q208 130 160 132 Q112 130 110 100 Z" stroke="var(--ink)" stroke-width="1.6" style="fill:var(--riddle-wash)"/>
<path d="M135 100 Q160 88 186 100" stroke="var(--faint)" stroke-width="1.2"/>
<text x="124" y="156" font-size="14" fill="var(--ink)" style="font-family:var(--font-voice)">chloroplast</text>
</g>
<g data-step="3" data-caption="Sugar and oxygen come out">
<path d="M214 98 Q250 94 288 98" stroke="var(--ink)" stroke-width="1.3" marker-end="url(#arrow)"/>
<text x="232" y="86" font-size="12" fill="var(--graphite)" style="font-family:var(--font-sans)">sugar + O2</text>
</g>
</svg>`;
}

export function cellDrawing(): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 320 210" style="fill:none;stroke-linecap:round;stroke-linejoin:round">
<g data-step="1" data-caption="A membrane holds the cell together">
<path d="M60 105 Q58 50 160 46 Q262 50 260 105 Q262 162 160 166 Q58 162 60 105 Z" stroke="var(--ink)" stroke-width="1.8" style="fill:var(--riddle-wash)"/>
<text x="118" y="30" font-size="14" fill="var(--ink)" style="font-family:var(--font-voice)">cell membrane</text>
</g>
<g data-step="2" data-caption="The nucleus keeps the instructions">
<path d="M130 90 Q132 68 158 68 Q184 70 184 92 Q182 114 156 114 Q130 112 130 90 Z" stroke="var(--ink)" stroke-width="1.6"/>
<path d="M146 90 Q158 82 170 92" stroke="var(--faint)" stroke-width="1.2"/>
<text x="124" y="136" font-size="13" fill="var(--graphite)" style="font-family:var(--font-sans)">nucleus</text>
</g>
<g data-step="3" data-caption="Mitochondria turn sugar into usable energy">
<path d="M206 118 Q206 104 222 104 Q240 104 240 118 Q240 132 222 132 Q206 132 206 118 Z" stroke="var(--ink)" stroke-width="1.5"/>
<path d="M212 118 Q218 110 224 120 Q230 126 236 116" stroke="var(--faint)" stroke-width="1.2"/>
<text x="196" y="152" font-size="13" fill="var(--graphite)" style="font-family:var(--font-sans)">mitochondrion</text>
</g>
<g data-step="4" data-caption="Together they keep the cell running">
<path d="M262 80 Q286 78 300 96" stroke="var(--ink)" stroke-width="1.5"/>
<path d="M300 96 L289 94 M300 96 L297 85" stroke="var(--ink)" stroke-width="1.5"/>
<text x="268" y="70" font-size="12" fill="var(--graphite)" style="font-family:var(--font-sans)">energy</text>
</g>
</svg>`;
}

export function drawingFor(topic: string): string {
  return /cell/i.test(topic) ? cellDrawing() : sampleDrawing();
}

export const KOKORO: VoiceOption[] = [
  ['bm_george', 'George', 'Warm, British'],
  ['bf_emma', 'Emma', 'Clear, British'],
  ['am_michael', 'Michael', 'Deep, steady'],
  ['af_heart', 'Heart', 'Warm, friendly'],
].map(([id, name, description]) => ({ id: `kokoro/${id}`, name, description, engine: 'Kokoro', source: 'Duragon', language: null }));

export const SPEECHD: VoiceOption[] = [
  ['pico', 'Pico', 'samantha', 'en-US'],
  ['pico', 'Pico', 'serena', 'en-GB'],
  ['pico', 'Pico', 'sabrina', 'de-DE'],
  ['espeak-ng', 'eSpeak NG', 'English (America)', 'en-us'],
  ['espeak-ng', 'eSpeak NG', 'English (Great Britain)', 'en-gb'],
  ['espeak-ng', 'eSpeak NG', 'French (France)', 'fr-fr'],
].map(([module, engine, name, language]) => ({
  id: `speechd/${module}/${name}`,
  name: name[0].toUpperCase() + name.slice(1),
  description: '',
  engine,
  source: 'Speech Dispatcher',
  language,
}));

export const PARAKEET: VoiceOption = {
  id: 'parakeet',
  name: 'Parakeet',
  description: 'English',
  engine: 'Parakeet',
  source: 'Duragon',
  language: null,
};

export const WHISPER: VoiceOption[] = [
  ['base.en', 'English'],
  ['small', 'Many languages'],
].map(([name, description]) => ({
  id: `whisper/home/you/.local/share/whisper.cpp/ggml-${name}.bin`,
  name,
  description,
  engine: 'whisper.cpp',
  source: 'on this computer',
  language: null,
}));

export function createSeedData(): {
  notes: Note[];
  revisions: Map<string, StoredRevision[]>;
  threads: Thread[];
} {
  const t = now();
  const welcome: Note = {
    id: uid(),
    title: 'Welcome to Duragon',
    folderId: null,
    updatedAt: t,
    hasOpenQuestion: true,
    doc: {
      type: 'doc',
      content: [
        title('Welcome to Duragon'),
        para('Duragon is a quiet notebook. You write, and Claude reads along. When something is worth a second look, it asks a question in the margin instead of interrupting.'),
        para('Address it by name and it answers right under your line. When a picture would help, it sketches one. You can also open the chat to talk about the whole note.'),
        para(`Your notes live on this machine. Try writing something wrong on purpose. ${QUOTE} Then see what comes back.`),
      ],
    },
    riddlerId: BUILTIN_RIDDLER_ID,
    mode: 'thinks',
    voiceOn: false,
    webOn: false,
    brief: '',
    createdAt: t,
  };
  const scratch: Note = {
    id: uid(),
    title: 'Scratch',
    folderId: null,
    updatedAt: t - 3_600_000,
    hasOpenQuestion: false,
    doc: { type: 'doc', content: [title('Scratch'), para('A second note, to fill the list.')] },
    riddlerId: BUILTIN_RIDDLER_ID,
    mode: 'thinks',
    voiceOn: false,
    webOn: false,
    brief: '',
    createdAt: t - 3_600_000,
  };
  const earlierWelcomeDoc = {
    type: 'doc',
    content: [
      title('Welcome to Duragon'),
      para('Duragon is a quiet notebook. You write, and Claude reads along in the margin.'),
      para('Your notes live on this machine. Open History whenever you want to look back at an earlier draft.'),
    ],
  };
  const revisions = new Map<string, StoredRevision[]>([
    [
      welcome.id,
      [
        {
          id: uid(),
          noteId: welcome.id,
          title: 'Welcome to Duragon',
          doc: earlierWelcomeDoc,
          plain: plainOf(earlierWelcomeDoc),
          createdAt: t - 25 * 60_000,
        },
      ],
    ],
  ]);
  const threadId = uid();
  const threads: Thread[] = [
    {
      id: threadId,
      noteId: welcome.id,
      kind: 'question',
      quote: QUOTE,
      status: 'open',
      pinned: false,
      createdAt: t,
      messages: [
        {
          id: uid(),
          noteId: welcome.id,
          threadId,
          author: 'riddled',
          text: 'Is that where it happens? What does the mitochondria do, and which part of the cell handles light?',
          drawing: 'none',
          svg: null,
          sources: [],
          symbols: [],
          trimmed: false,
          createdAt: t,
        },
      ],
    },
  ];
  return { notes: [welcome, scratch], revisions, threads };
}
