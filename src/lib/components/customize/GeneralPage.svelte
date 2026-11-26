<script lang="ts">
  import type { Settings } from '../../api/types';
  import { settings, updateSettings } from '../../state/settings.svelte';
  import Choice from './Choice.svelte';
  import PlacementPreview from './PlacementPreview.svelte';
  import SwitchRow from './SwitchRow.svelte';

  const s = $derived(settings.value);
  const set = (patch: Partial<Settings>) => void updateSettings(patch);
</script>

<section>
  <h3>Reading</h3>
  <p class="sub">When Duragon reads along, and where its thoughts go.</p>

  <div class="group">
    <h4 id="g-start">New notes start in</h4>
    <div class="grid two" role="radiogroup" aria-labelledby="g-start">
      <Choice selected={s.newNoteMode === 'thinks'} onpick={() => set({ newNoteMode: 'thinks' })} title="Thinks with you" desc="Reads along and speaks up when something’s worth a second look." />
      <Choice selected={s.newNoteMode === 'asks'} onpick={() => set({ newNoteMode: 'asks' })} title="When you ask" desc="Quiet until you say its name, select words, or open the chat." />
    </div>
    <p class="note">Change any note on its own, from the row above its title.</p>
  </div>

  <div class="group">
    <h4 id="g-pace">Reading pace</h4>
    <div class="grid two" role="radiogroup" aria-labelledby="g-pace">
      <Choice selected={s.readingPace === 'regular'} onpick={() => set({ readingPace: 'regular' })} title="Regular" desc="Reads a moment after you stop typing." />
      <Choice selected={s.readingPace === 'slow'} onpick={() => set({ readingPace: 'slow' })} title="Slow" desc="Waits about twice as long, for when you pause mid-thought." />
    </div>
  </div>

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

  <div class="group">
    <SwitchRow title="Web search in new notes" desc="Lets Duragon look things up, and shows where it looked." on={s.webInNewNotes} onchange={(v) => set({ webInNewNotes: v })} />
  </div>
  <div class="group">
    <SwitchRow title="Search my other notes in chat" desc="In the side chat, Duragon can look through your other notes when it helps, and says which one it used. The notes it opens go to the model you picked for Chat, like the note you’re in." on={s.searchMyNotes} onchange={(v) => set({ searchMyNotes: v })} />
  </div>
  <div class="group">
    <SwitchRow title="Explain symbols in formulas" desc="Point at a symbol in a formula Duragon writes to see what it stands for. Click it to ask for more." on={s.explainSymbols} onchange={(v) => set({ explainSymbols: v })} />
  </div>
</section>

<section>
  <h3>Notes</h3>
  <p class="sub">What happens to the notes you delete.</p>
  <div class="group">
    <h4 id="g-deleted">Deleted notes</h4>
    <div class="grid three" role="radiogroup" aria-labelledby="g-deleted">
      <Choice selected={s.deletedNotes === 'duragon'} onpick={() => set({ deletedNotes: 'duragon' })} title="Duragon’s trash" desc="Kept in .trash until you empty it." />
      <Choice selected={s.deletedNotes === 'system'} onpick={() => set({ deletedNotes: 'system' })} title="System trash" desc="Moved to your computer’s trash. Restore it there, and it shows in Duragon’s Trash." />
      <Choice selected={s.deletedNotes === 'permanent'} onpick={() => set({ deletedNotes: 'permanent' })} title="Delete for good" desc="Asks first. Nothing is kept." />
    </div>
  </div>
</section>

<style>
  section { margin-top: 34px; }
  section:first-child { margin-top: 0; }
  h3 { margin: 0; font-size: 16px; line-height: 1.3; font-weight: 500; }
  .sub { margin-top: 3px; font-size: 13.5px; line-height: 1.45; color: var(--faint); }
  .group { margin-top: 22px; }
  h4 { margin: 0 0 10px; font-size: 13.5px; line-height: 1.3; font-weight: 500; color: var(--ink); }
  .grid { display: grid; gap: 10px; }
  .two { grid-template-columns: 1fr 1fr; }
  .three { grid-template-columns: repeat(3, 1fr); }
  .note { margin-top: 8px; font-size: 13px; line-height: 1.4; color: var(--faint); }
</style>
