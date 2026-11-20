<script lang="ts">
  let { on, onchange, label }: { on: boolean; onchange: (v: boolean) => void; label: string } = $props();
</script>

<button class="switch" class:on role="switch" aria-checked={on} aria-label={label} onclick={() => onchange(!on)}>
  <span class="knob"></span>
</button>

<style>
  .switch {
    position: relative;
    flex: none;
    width: 40px;
    height: 24px;
    border-radius: 999px;
    background: var(--rule);
    transition: background-color 180ms var(--ease-ui);
  }
  .knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--faint);
    transition: transform 220ms var(--ease-step), background-color 180ms var(--ease-ui);
  }
  .on { background: var(--ink); }
  .on .knob { transform: translateX(16px); background: var(--paper); }
</style>
