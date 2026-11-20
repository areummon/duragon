<script lang="ts">
  let { on }: { on: boolean } = $props();
</script>

<span class="track" class:on aria-hidden="true"><span class="knob"></span></span>

<style>
  .track {
    position: relative;
    flex: none;
    width: 24px;
    height: 14px;
    border-radius: 999px;
    background: var(--rule);
    transition: background-color 180ms var(--ease-ui);
  }
  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--faint);
    transition: transform 220ms var(--ease-step), background-color 180ms var(--ease-ui);
  }
  .on { background: var(--graphite); }
  .on .knob { transform: translateX(10px); background: var(--paper); }
</style>
