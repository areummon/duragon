<script lang="ts">
  import { settings } from '../state/settings.svelte';
  import { blotSvg } from '../ui/blot';

  const reduced = () => matchMedia('(prefers-reduced-motion: reduce)').matches;

  $effect(() => {
    if (!settings.value.inkFromPointer || reduced()) return;
    let moved = 0;
    let last = 0;
    let next = 4000 + Math.random() * 3000;

    const drop = (x: number, y: number) => {
      const el = document.createElement('div');
      el.className = 'ink-drop';
      const size = 10 + Math.random() * 8;
      el.style.cssText = `left:${x - size / 2}px;top:${y - size / 2}px;width:${size}px;height:${size}px`;
      el.innerHTML = blotSvg(`drop-${Math.random()}`, size);
      document.body.appendChild(el);
      el.animate([{ transform: 'scale(0.12)' }, { transform: 'scale(1)' }], { duration: 460, easing: 'cubic-bezier(0.2, 0.9, 0.25, 1.12)', fill: 'both' });
      const fade = el.animate([{ opacity: 0.7 }, { opacity: 0.35, offset: 0.3 }, { opacity: 0 }], { duration: 3000, easing: 'ease-in', fill: 'both' });
      fade.onfinish = () => el.remove();
    };

    const move = (e: PointerEvent) => {
      const t = performance.now();
      if (last) moved += Math.min(t - last, 100);
      last = t;
      if (moved >= next) {
        moved = 0;
        next = 4000 + Math.random() * 3000;
        drop(e.clientX, e.clientY);
      }
    };
    window.addEventListener('pointermove', move, { passive: true });
    return () => window.removeEventListener('pointermove', move);
  });
</script>
