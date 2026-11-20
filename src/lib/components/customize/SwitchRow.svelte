<script lang="ts">
  import Switch from './Switch.svelte';

  let { title, desc, on, onchange }: { title: string; desc?: string; on: boolean; onchange: (v: boolean) => void } = $props();
</script>

<div class="row">
  <div class="text">
    <div class="title">{title}</div>
    {#if desc}<div class="desc">{desc}</div>{/if}
  </div>
  <Switch {on} {onchange} label={title} />
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 20px;
    padding: 14px 16px;
    border: 1px solid var(--rule);
    border-radius: 12px;
  }
  .text { flex: 1; min-width: 0; }
  .title { font-size: 13.5px; line-height: 1.3; font-weight: 500; }
  .desc { margin-top: 3px; font-size: 13px; line-height: 1.4; color: var(--faint); }
</style>
