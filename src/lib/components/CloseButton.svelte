<script lang="ts">
  import X from 'lucide-svelte/icons/x';

  interface Props {
    onclick: () => void;
    label?: string;
    class?: string;
  }
  let { onclick, label = 'Close', class: className = '' }: Props = $props();
</script>

<button type="button" class="close-btn {className}" aria-label={label} {onclick}>
  <X size={17} strokeWidth={1.5} />
</button>

<style>
  .close-btn {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 1px solid var(--rule);
    border-radius: 8px;
    background: var(--shelf);
    color: var(--graphite);
    transition:
      color 150ms var(--ease-ui),
      background-color 150ms var(--ease-ui),
      border-color 150ms var(--ease-ui),
      transform 140ms var(--ease-step);
  }
  @media (hover: hover) and (pointer: fine) {
    .close-btn:hover {
      color: var(--ink);
      background: var(--riddle-wash);
      border-color: color-mix(in srgb, var(--ink) 20%, var(--rule));
    }
  }
  .close-btn:active {
    transform: scale(0.95);
  }
  @media (prefers-reduced-motion: reduce) {
    .close-btn:active {
      transform: none;
    }
  }
</style>
