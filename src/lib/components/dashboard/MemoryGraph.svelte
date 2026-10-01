<script lang="ts">
  import type { VectorHit } from '$lib/api/types';

  let { hits = [] }: { hits?: VectorHit[] } = $props();

  let canvas: HTMLCanvasElement | undefined = $state();

  function draw() {
    const ctx = canvas?.getContext('2d');
    if (!ctx || !canvas || hits.length === 0) return;

    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);

    const cx = w / 2;
    const cy = h / 2;
    const maxR = Math.min(w, h) * 0.4;

    hits.forEach((hit, i) => {
      // Lay hits out on a golden-angle spiral: stable and non-overlapping.
      const a = i * 2.399963;
      const similarity = 1 - hit.distance / 2;
      const r = maxR * (0.25 + 0.75 * similarity);
      const x = cx + Math.cos(a) * r;
      const y = cy + Math.sin(a) * r;
      const size = 6 + similarity * 14;

      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(x, y);
      ctx.strokeStyle = 'rgba(255, 107, 53, 0.22)';
      ctx.lineWidth = 1;
      ctx.stroke();

      ctx.beginPath();
      ctx.arc(x, y, size, 0, Math.PI * 2);
      ctx.fillStyle = hit.memory_type === 'conversation' ? '#ff6b35' : '#3498db';
      ctx.globalAlpha = 0.75;
      ctx.fill();
      ctx.globalAlpha = 1;
    });

    ctx.beginPath();
    ctx.arc(cx, cy, 5, 0, Math.PI * 2);
    ctx.fillStyle = '#fff';
    ctx.fill();
  }

  $effect(() => {
    void hits;
    draw();
  });

  // First paint can land before layout gives the canvas its size; redraw
  // whenever the element itself is resized.
  $effect(() => {
    const el = canvas;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(() => draw());
    ro.observe(el);
    return () => ro.disconnect();
  });
</script>

<canvas bind:this={canvas} aria-label="语义记忆分布"></canvas>

<style>
  canvas { display: block; width: 100%; max-width: 300px; height: auto; margin: 0 auto; aspect-ratio: 1; }
</style>
