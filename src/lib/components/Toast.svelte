<script lang="ts">
  import { rise } from '../motion/transitions';
  import { app } from '../state/app.svelte';
  import { hideToast, toast } from '../state/toast.svelte';
</script>

{#if toast.text}
  {#key toast.seq}
    <div class="toast" class:beside={app.sidebarOpen} role="status" transition:rise={{ y: 6, duration: 200 }}>
      <span>{toast.text}</span>
      {#if toast.action}
        <button onclick={() => { toast.action?.run(); hideToast(); }}>{toast.action.label}</button>
      {/if}
    </div>
  {/key}
{/if}

<style>
  .toast {
    position: fixed;
    left: 24px;
    bottom: 20px;
    z-index: 80;
    display: flex;
    align-items: center;
    gap: 14px;
    max-width: min(440px, calc(100vw - 48px));
    padding: 9px 10px 9px 16px;
    border: 1px solid var(--rule);
    border-radius: 14px;
    background: var(--paper);
    box-shadow: var(--lift);
    font-size: 13.5px;
    line-height: 1.4;
    color: var(--graphite);
  }
  /* Clear of the sidebar's last rows. */
  @media (min-width: 768px) {
    .toast.beside { left: 312px; max-width: min(440px, calc(100vw - 336px)); }
  }
  button {
    flex: none;
    height: 28px;
    padding: 0 12px;
    border-radius: 999px;
    background: var(--ink);
    color: var(--paper);
    font-size: 13px;
    transition: transform 160ms var(--ease-step);
  }
  /* Gives a little under the press. */
  button:active:not(:disabled) { transform: scale(0.97); }
  @media (prefers-reduced-motion: reduce) { button:active:not(:disabled) { transform: none; } }
</style>
