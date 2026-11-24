<script lang="ts" generics="T extends string">
  interface Props {
    options: { id: T; name: string }[];
    value: T;
    onpick: (id: T) => void;
    labelledby: string;
    small?: boolean;
  }
  let { options, value, onpick, labelledby, small = false }: Props = $props();

  let group = $state<HTMLElement>();
  const at = $derived(Math.max(0, options.findIndex((o) => o.id === value)));

  const STEP: Record<string, number> = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 };
  function onkeydown(e: KeyboardEvent) {
    const step = STEP[e.key];
    if (!step || e.ctrlKey || e.altKey || e.metaKey) return;
    e.preventDefault();
    const next = (at + step + options.length) % options.length;
    onpick(options[next].id);
    (group?.children[next] as HTMLElement).focus();
  }
</script>

<div class="pills" class:small role="radiogroup" aria-labelledby={labelledby} tabindex="-1" bind:this={group} {onkeydown}>
  {#each options as o, i (o.id)}
    <button type="button" class:on={i === at} role="radio" aria-checked={i === at} tabindex={i === at ? 0 : -1} onclick={() => onpick(o.id)}>{o.name}</button>
  {/each}
</div>

<style>
  .pills { display: flex; flex-wrap: wrap; gap: 6px; outline: none; }
  .pills.small { gap: 4px; flex-wrap: nowrap; }
  button {
    height: 30px;
    padding: 0 12px;
    border: 1px solid var(--rule);
    border-radius: 999px;
    font-size: 13px;
    line-height: 1;
    color: var(--ink);
    transition: background-color 150ms var(--ease-ui), color 150ms var(--ease-ui), border-color 150ms var(--ease-ui);
  }
  .small button { flex: 1 1 auto; height: 28px; padding: 0 8px; font-size: 12.5px; }
  @media (hover: hover) and (pointer: fine) { button:not(.on):hover { border-color: var(--faint); } }
  button.on { background: var(--ink); border-color: var(--ink); color: var(--paper); }
</style>
