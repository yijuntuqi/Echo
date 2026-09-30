<script lang="ts">
  import { moodStore } from '$lib/stores/mood';

  const CELL = 13;
  const GAP = 3;

  const COLORS: Record<string, string> = {
    happy: '#2ecc71', sad: '#3498db', anxious: '#f39c12', calm: '#1abc9c',
    angry: '#e74c3c', excited: '#e67e22', bored: '#95a5a6', lonely: '#9b59b6',
    grateful: '#f1c40f', neutral: '#bdc3c7',
  };

  let canvas: HTMLCanvasElement | undefined = $state();

  function draw() {
    const ctx = canvas?.getContext('2d');
    if (!ctx || !canvas) return;

    const dpr = window.devicePixelRatio || 1;
    const entries = moodStore.entries;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);

    if (entries.length === 0) return;

    // Anchor the grid to the earliest entry so early data is not pushed off-screen.
    const dates = entries.map((e) => new Date(e.date).getTime());
    const min = Math.min(...dates);
    const start = new Date(min);
    start.setHours(0, 0, 0, 0);
    // Snap the grid start back to Sunday.
    start.setDate(start.getDate() - start.getDay());

    const byDay = new Map(entries.map((e) => [e.date, e]));

    for (let t = start.getTime(); t <= Date.now(); t += 864e5 * 7) {
      const week = Math.floor((t - start.getTime()) / (864e5 * 7));
      for (let dow = 0; dow < 7; dow++) {
        const day = new Date(t + dow * 864e5);
        const iso = day.toISOString().slice(0, 10);
        const entry = byDay.get(iso);

        const x = week * (CELL + GAP);
        const y = dow * (CELL + GAP);
        if (x + CELL > w) continue;

        ctx.fillStyle = COLORS[entry?.emotion ?? 'neutral'] ?? COLORS.neutral;
        ctx.globalAlpha = entry ? Math.min(1, Math.max(0.25, entry.weight)) : 0.12;
        roundRect(ctx, x, y, CELL, CELL, 3);
        ctx.fill();
      }
    }
    ctx.globalAlpha = 1;
  }

  function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
    ctx.beginPath();
    ctx.roundRect(x, y, w, h, r);
  }

  $effect(() => {
    void moodStore.entries;
    draw();
  });
</script>

<div class="wrap">
  <canvas bind:this={canvas} aria-label="心情热力图"></canvas>
  {#if moodStore.entries.length === 0}
    <p class="empty">还没有心情记录</p>
  {/if}
</div>

<style>
  .wrap { width: 100%; overflow-x: auto; }
  .wrap canvas { display: block; width: 100%; height: 7 * 16px; }
  .empty { text-align: center; color: var(--color-text-muted); font-size: 13px; margin-top: 8px; }
</style>
