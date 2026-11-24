<script lang="ts">
  import Check from 'lucide-svelte/icons/check';
  import Play from 'lucide-svelte/icons/play';
  import type { Settings } from '../../api/types';
  import { settings, updateSettings } from '../../state/settings.svelte';
  import { readAloud, voice } from '../../state/voice.svelte';
  import Choice from './Choice.svelte';
  import PlacementPreview from './PlacementPreview.svelte';
  import SwitchRow from './SwitchRow.svelte';

  let { query = '' }: { query?: string } = $props();

  const s = $derived(settings.value);
  const q = $derived(query.trim().toLowerCase());
  const set = (patch: Partial<Settings>) => void updateSettings(patch);

  const SECTIONS = {
    reading: 'reading when duragon reads along, and where its thoughts go',
    voice: 'voice speak aloud speaking sound',
    look: 'look appearance theme colour color lettering text size page width ink',
    models: 'models which claude model does what',
  } as const;
  const ROWS: Record<string, string> = {
    start: 'new notes start in thinks with you when you ask reading mode',
    pace: 'reading pace regular slow',
    place: 'where questions appear both margins right margin marks only',
    web: 'web search in new notes',
    notes: 'search my other notes in chat recall past earlier',
    symbols: 'explain symbols in formulas math latex equation variable meaning',
    speak: 'new notes speak aloud voice',
    pick: 'voice george emma michael heart sample',
    speed: 'speaking speed slower faster',
    appearance: 'appearance system light dark theme',
    size: 'text size small default large',
    width: 'page width reset',
    lettering: "lettering duragon's own like your notes font",
    colour: 'colour color ink blue green red purple brown orange',
    pointer: 'ink from the pointer drops',
    readModel: 'reading model',
    chatModel: 'chat model',
    drawModel: 'drawing model',
  };
  const GROUPS: Record<keyof typeof SECTIONS, string[]> = {
    reading: ['start', 'pace', 'place', 'web', 'notes', 'symbols'],
    voice: ['speak', 'pick', 'speed'],
    look: ['appearance', 'size', 'width', 'lettering', 'colour', 'pointer'],
    models: ['readModel', 'chatModel', 'drawModel'],
  };
  const secOn = (k: keyof typeof SECTIONS) => !q || SECTIONS[k].includes(q) || GROUPS[k].some((r) => ROWS[r].includes(q));
  const on = (k: keyof typeof SECTIONS, row: string) => !q || SECTIONS[k].includes(q) || ROWS[row].includes(q);
  const empty = $derived(!!q && !(Object.keys(SECTIONS) as (keyof typeof SECTIONS)[]).some(secOn));

  const INKS = [
    ['ink', 'Ink', 'var(--ink)'],
    ['blue', 'Blue', 'var(--text-blue)'],
    ['green', 'Green', 'var(--text-green)'],
    ['red', 'Red', 'var(--text-red)'],
    ['purple', 'Purple', 'var(--text-purple)'],
    ['brown', 'Brown', 'var(--text-brown)'],
    ['orange', 'Orange', 'var(--text-orange)'],
  ] as const;

  const selectedVoice = $derived(voice.voices.some((v) => v.id === s.voiceName) ? s.voiceName : voice.voices[0]?.id);

  async function sample(id: string) {
    await updateSettings({ voiceName: id });
    await readAloud(null, 'Does this still hold?');
  }

  const MODELS = ['haiku', 'sonnet', 'opus', 'fable'];
  function commitModel(key: 'readModel' | 'chatModel' | 'drawModel', e: Event) {
    const v = (e.currentTarget as HTMLInputElement).value.trim();
    if (v && v !== s[key]) set({ [key]: v });
  }
</script>

{#if secOn('reading')}
  <section>
    <h3>Reading</h3>
    <p class="sub">When Duragon reads along, and where its thoughts go.</p>

    {#if on('reading', 'start')}
      <div class="group">
        <h4 id="g-start">New notes start in</h4>
        <div class="grid two" role="radiogroup" aria-labelledby="g-start">
          <Choice selected={s.newNoteMode === 'thinks'} onpick={() => set({ newNoteMode: 'thinks' })} title="Thinks with you" desc="Reads along and speaks up when something’s worth a second look." />
          <Choice selected={s.newNoteMode === 'asks'} onpick={() => set({ newNoteMode: 'asks' })} title="When you ask" desc="Quiet until you say its name, select words, or open the chat." />
        </div>
        <p class="note">Change any note on its own, from the row above its title.</p>
      </div>
    {/if}
    {#if on('reading', 'pace')}
      <div class="group">
        <h4 id="g-pace">Reading pace</h4>
        <div class="grid two" role="radiogroup" aria-labelledby="g-pace">
          <Choice selected={s.readingPace === 'regular'} onpick={() => set({ readingPace: 'regular' })} title="Regular" desc="Reads a moment after you stop typing." />
          <Choice selected={s.readingPace === 'slow'} onpick={() => set({ readingPace: 'slow' })} title="Slow" desc="Waits about twice as long, for when you pause mid-thought." />
        </div>
      </div>
    {/if}
    {#if on('reading', 'place')}
      <div class="group">
        <h4 id="g-place">Where questions appear</h4>
        <div class="grid three" role="radiogroup" aria-labelledby="g-place">
          <Choice selected={s.questionPlacement === 'both'} onpick={() => set({ questionPlacement: 'both' })} title="Both margins" desc="Your words in the middle, questions on either side.">
            {#snippet preview()}<PlacementPreview kind="both" />{/snippet}
          </Choice>
          <Choice selected={s.questionPlacement === 'right'} onpick={() => set({ questionPlacement: 'right' })} title="Right margin" desc="Every question in one column on the right.">
            {#snippet preview()}<PlacementPreview kind="right" />{/snippet}
          </Choice>
          <Choice selected={s.questionPlacement === 'marks'} onpick={() => set({ questionPlacement: 'marks' })} title="Marks only" desc="A small mark after the passage. Open it to read.">
            {#snippet preview()}<PlacementPreview kind="marks" />{/snippet}
          </Choice>
        </div>
      </div>
    {/if}
    {#if on('reading', 'web')}
      <div class="group"><SwitchRow title="Web search in new notes" desc="Lets Duragon look things up, and shows where it looked." on={s.webInNewNotes} onchange={(v) => set({ webInNewNotes: v })} /></div>
    {/if}
    {#if on('reading', 'notes')}
      <div class="group"><SwitchRow title="Search my other notes in chat" desc="In the side chat, Duragon can look through your other notes when it helps, and says which one it used. The notes it opens are sent to Claude, like the note you’re in." on={s.searchMyNotes} onchange={(v) => set({ searchMyNotes: v })} /></div>
    {/if}
    {#if on('reading', 'symbols')}
      <div class="group"><SwitchRow title="Explain symbols in formulas" desc="Point at a symbol in a formula Duragon writes to see what it stands for. Click it to ask for more." on={s.explainSymbols} onchange={(v) => set({ explainSymbols: v })} /></div>
    {/if}
  </section>
{/if}

{#if secOn('voice')}
  <section>
    <h3>Voice</h3>
    <p class="sub">How Duragon sounds when it speaks.</p>
    {#if on('voice', 'speak')}
      <div class="group"><SwitchRow title="New notes speak aloud" desc="Each new thought is read out once as it’s written. Typing stops it." on={s.voiceInNewNotes} onchange={(v) => set({ voiceInNewNotes: v })} /></div>
    {/if}
    {#if on('voice', 'pick')}
      <div class="group">
        <h4 id="g-voice">Voice</h4>
        <div class="voices" role="radiogroup" aria-labelledby="g-voice">
          {#each voice.voices as v (v.id)}
            <div class="vrow" class:selected={v.id === selectedVoice}>
              <button class="pick" role="radio" aria-checked={v.id === selectedVoice} onclick={() => set({ voiceName: v.id })}>
                <span class="tick">{#if v.id === selectedVoice}<Check size={14} strokeWidth={1.8} />{/if}</span>
                <span class="vname">{v.name}</span><span class="vdesc">{v.description}</span>
              </button>
              <button class="play" aria-label="Play a sample of {v.name}" title="Play a sample" onclick={() => void sample(v.id)}><Play size={13} strokeWidth={1.6} /></button>
            </div>
          {/each}
        </div>
      </div>
    {/if}
    {#if on('voice', 'speed')}
      <div class="group">
        <div class="speedhead"><h4 id="g-speed">Speaking speed</h4><output class="val">{s.speakingSpeed.toFixed(2)}×</output></div>
        <input
          class="range"
          type="range"
          min="0.5"
          max="2"
          step="0.05"
          aria-labelledby="g-speed"
          value={s.speakingSpeed}
          style:--fill="{((s.speakingSpeed - 0.5) / 1.5) * 100}%"
          oninput={(e) => set({ speakingSpeed: Number(e.currentTarget.value) })}
          ondblclick={() => set({ speakingSpeed: 1 })}
        />
        <div class="ends"><span>Slower</span><span>Faster</span></div>
      </div>
    {/if}
  </section>
{/if}

{#if secOn('look')}
  <section>
    <h3>Look</h3>
    <p class="sub">How the page and Duragon’s ink look.</p>
    {#if on('look', 'appearance')}
      <div class="group">
        <h4 id="g-app">Appearance</h4>
        <div class="grid three" role="radiogroup" aria-labelledby="g-app">
          {#each [['system', 'System'], ['light', 'Light'], ['dark', 'Dark']] as const as [id, name] (id)}
            <Choice selected={s.appearance === id} onpick={() => set({ appearance: id })} title={name} />
          {/each}
        </div>
      </div>
    {/if}
    {#if on('look', 'size')}
      <div class="group">
        <h4 id="g-size">Text size</h4>
        <div class="grid three" role="radiogroup" aria-labelledby="g-size">
          {#each [['small', 'Small', 15], ['default', 'Default', 16], ['large', 'Large', 18]] as const as [id, name, px] (id)}
            <Choice selected={s.textSize === id} onpick={() => set({ textSize: id })} title={name}>
              <span class="sample" style:font-size="{px}px">A thought.</span>
            </Choice>
          {/each}
        </div>
      </div>
    {/if}
    {#if on('look', 'width')}
      <div class="group">
        <div class="line">
          <div><h4>Page width</h4><p class="note tight">Drag the two ticks above the page to change it.</p></div>
          <button class="ghost" disabled={s.pageWidth === null} onclick={() => set({ pageWidth: null })}>Reset</button>
        </div>
      </div>
    {/if}
    {#if on('look', 'lettering')}
      <div class="group">
        <h4 id="g-let">Duragon’s lettering</h4>
        <div class="grid two" role="radiogroup" aria-labelledby="g-let">
          <Choice selected={s.lettering === 'own'} onpick={() => set({ lettering: 'own' })} title="Duragon’s own">
            <span class="sample serif">Does this still hold?</span>
          </Choice>
          <Choice selected={s.lettering === 'notes'} onpick={() => set({ lettering: 'notes' })} title="Like your notes">
            <span class="sample sans">Does this still hold?</span>
          </Choice>
        </div>
      </div>
    {/if}
    {#if on('look', 'colour')}
      <div class="group">
        <h4 id="g-col">Duragon’s colour</h4>
        <div class="swatches" role="radiogroup" aria-labelledby="g-col">
          {#each INKS as [id, name, colour] (id)}
            <button class="swatch" class:selected={s.inkColour === id} role="radio" aria-checked={s.inkColour === id} aria-label={name} title={name} onclick={() => set({ inkColour: id })}>
              <span style:background={colour}></span>
            </button>
          {/each}
        </div>
      </div>
    {/if}
    {#if on('look', 'pointer')}
      <div class="group"><SwitchRow title="Ink from the pointer" desc="Now and then a drop of ink falls as you move the pointer, then dries and fades." on={s.inkFromPointer} onchange={(v) => set({ inkFromPointer: v })} /></div>
    {/if}
  </section>
{/if}

{#if secOn('models')}
  <section>
    <h3>Models</h3>
    <p class="sub">Which Claude model does each job. Pick one, or type any model name.</p>
    {#each [['readModel', 'Reading model', 'Reads along and asks questions.'], ['chatModel', 'Chat model', 'Answers in the chat and when you address it.'], ['drawModel', 'Drawing model', 'Sketches the drawings.']] as const as [key, name, desc] (key)}
      {#if on('models', key)}
        <div class="group">
          <div class="line">
            <div><h4>{name}</h4><p class="note tight">{desc}</p></div>
            <input class="model" list="models" value={s[key]} aria-label={name} spellcheck="false" onchange={(e) => commitModel(key, e)} />
          </div>
        </div>
      {/if}
    {/each}
    <datalist id="models">{#each MODELS as m (m)}<option value={m}></option>{/each}</datalist>
  </section>
{/if}

{#if empty}<p class="none">Nothing matches “{query}”.</p>{/if}

<style>
  section { margin-top: 34px; }
  section:first-child { margin-top: 28px; }
  h3 { margin: 0; font-size: 17px; line-height: 1.3; font-weight: 500; }
  .sub { margin-top: 3px; font-size: 13.5px; line-height: 1.45; color: var(--faint); }
  .group { margin-top: 22px; }
  h4 { margin: 0 0 10px; font-size: 13.5px; line-height: 1.3; font-weight: 500; color: var(--ink); }
  .line h4 { margin-bottom: 0; }
  .grid { display: grid; gap: 10px; }
  .two { grid-template-columns: 1fr 1fr; }
  .three { grid-template-columns: repeat(3, 1fr); }
  .note { margin-top: 8px; font-size: 13px; line-height: 1.4; color: var(--faint); }
  .note.tight { margin-top: 3px; }
  .sample { margin-top: 4px; line-height: 1.3; color: var(--ink); }
  .sample.serif { font-family: 'Newsreader Variable', Georgia, serif; font-size: 16px; }
  .sample.sans { font-family: var(--font-sans); font-size: 15px; }

  .voices { border: 1px solid var(--rule); border-radius: 12px; overflow: hidden; }
  .vrow { display: flex; align-items: center; border-top: 1px solid var(--rule); }
  .vrow:first-child { border-top: 0; }
  .pick { flex: 1; display: flex; align-items: center; gap: 10px; min-width: 0; padding: 11px 6px 11px 14px; text-align: left; font-size: 13.5px; }
  .tick { display: grid; place-items: center; width: 16px; flex: none; color: var(--ink); }
  .vname { font-weight: 500; }
  .vdesc { color: var(--faint); }
  .play { display: grid; place-items: center; width: 30px; height: 30px; margin-right: 10px; border-radius: 50%; color: var(--graphite); transition: background-color 150ms var(--ease-ui), color 150ms var(--ease-ui); }
  .play:hover { background: var(--riddle-wash); color: var(--ink); }

  .speedhead { display: flex; justify-content: space-between; align-items: baseline; }
  .speedhead h4 { margin-bottom: 8px; }
  .val { font-size: 13px; color: var(--graphite); font-variant-numeric: tabular-nums; }
  .range { width: 100%; margin: 4px 0 0; height: 20px; background: transparent; appearance: none; cursor: var(--cursor-pointer); }
  .range::-webkit-slider-runnable-track { height: 2px; border-radius: 1px; background: linear-gradient(to right, var(--ink) var(--fill), var(--rule) var(--fill)); }
  .range::-webkit-slider-thumb { appearance: none; width: 16px; height: 16px; margin-top: -7px; border-radius: 50%; background: var(--paper); border: 1.5px solid var(--ink); }
  .range::-moz-range-track { height: 2px; background: var(--rule); }
  .range::-moz-range-progress { height: 2px; background: var(--ink); }
  .range::-moz-range-thumb { width: 13px; height: 13px; border-radius: 50%; background: var(--paper); border: 1.5px solid var(--ink); }
  .ends { display: flex; justify-content: space-between; margin-top: 2px; font-size: 12.5px; color: var(--faint); }

  .line { display: flex; align-items: center; justify-content: space-between; gap: 20px; }
  .ghost { height: 30px; padding: 0 14px; border: 1px solid var(--rule); border-radius: 999px; font-size: 13px; transition: opacity 150ms var(--ease-ui), transform 160ms var(--ease-step); }
  .ghost:disabled { opacity: 0.4; cursor: default; }
  .model { width: 170px; height: 34px; padding: 0 12px; border: 1px solid var(--rule); border-radius: 9px; background: transparent; font: inherit; font-size: 13.5px; color: var(--ink); outline: none; }
  .model:focus { border-color: var(--faint); }

  .swatches { display: flex; gap: 12px; }
  .swatch { display: grid; place-items: center; width: 34px; height: 34px; border-radius: 50%; border: 1.5px solid transparent; transition: border-color 150ms var(--ease-ui); }
  .swatch span { width: 20px; height: 20px; border-radius: 50%; }
  .swatch.selected { border-color: var(--ink); }
  .none { margin-top: 40px; font-size: 14px; color: var(--faint); }
  /* Gives a little under the press. */
  .ghost:active:not(:disabled) { transform: scale(0.97); }
  @media (prefers-reduced-motion: reduce) { .ghost:active:not(:disabled) { transform: none; } }
</style>
