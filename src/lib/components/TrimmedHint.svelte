<script lang="ts">
  let { around }: { around: 'cursor' | 'passage' } = $props();
</script>

<p class="trimmed">Read around {around === 'cursor' ? 'your cursor' : 'this passage'}; the note was too long to read whole.</p>

<style>
  .trimmed { margin-top: 6px; font-family: var(--font-sans); font-size: 12.5px; line-height: 1.4; color: var(--faint); }
</style>
