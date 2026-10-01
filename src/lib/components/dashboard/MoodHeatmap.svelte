<script lang="ts">
  import type { MoodEntry } from '$lib/api/types';
  import { moodStore } from '$lib/stores/mood';

  const CELL = 13;
  const GAP = 3;
  const STEP = CELL + GAP;

  const COLORS: Record<string, string> = {
    happy: '#2ecc71', sad: '#3498db', anxious: '#f39c12', calm: '#1abc9c',
    angry: '#e74c3c', excited: '#e67e22', bored: '#95a5a6', lonely: '#9b59b6',
    grateful: '#f1c40f', neutral: '#bdc3c7',
  };

  let canvas: HTMLCanvasElement | undefined = $state();
  let byDay = new Map<string, MoodEntry>();
  let gridStart = 0;

  /** Local-time YYYY-MM-DD. Mood rows are stored with localtime dates, and
   * toISOString() would render UTC — one day off in UTC+8 afternoons. */
  function localIso(d: Date): string {
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${d.getFullYear()}-${m}-${day}`;
  }

  function draw() {
    const ctx = canvas?.getContext('2d');
    if (!ctx || !canvas) return;

    const dpr = window.devicePixelRatio || 1;
    const entries = moodStore.entries;
    // Height in real pixels: `7 * 16px` is not valid CSS and was ignored,
    // leaving the canvas at its intrinsic 150px.
    const h = 7 * STEP - GAP;
    canvas.style.height = `${h}px`;

    // Size the canvas to the data, not the container: the wrap scrolls
    // horizontally, so no cell is ever clipped on the right.
    let weeks = 1;
    if (entries.length > 0) {
      const dates = entries.map((e) => new Date(e.date).getTime());
      const start = new Date(Math.min(...dates));
      start.setHours(0, 0, 0, 0);
      // Snap the grid start back to Sunday.
      start.setDate(start.getDate() - start.getDay());
      gridStart = start.getTime();
      weeks = Math.ceil((Date.now() - gridStart) / (864e5 * 7)) + 1;
      byDay = new Map(entries.map((e) => [e.date, e]));
    } else {
      byDay = new Map();
      gridStart = 0;
    }
    const contentW = Math.max(weeks * STEP, canvas.parentElement?.clientWidth ?? 0);
    canvas.style.minWidth = `${contentW}px`;
    canvas.width = contentW * dpr;
    canvas.height = h * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, contentW, h);

    if (entries.length === 0) return;

    for (let week = 0; week < weeks; week++) {
      for (let dow = 0; dow < 7; dow++) {
        const day = new Date(gridStart + (week * 7 + dow) * 864e5);
        if (day.getTime() > Date.now()) break;
        const entry = byDay.get(localIso(day));

        const x = week * STEP;
        const y = dow * STEP;

        ctx.fillStyle = COLORS[entry?.emotion ?? 'neutral'] ?? COLORS.neutral;
        ctx.globalAlpha = entry ? Math.min(1, Math.max(0.25, entry.weight)) : 0.12;
        ctx.beginPath();
        ctx.roundRect(x, y, CELL, CELL, 3);
        ctx.fill();
      }
    }
    ctx.globalAlpha = 1;
  }

  $effect(() => {
    void moodStore.entries;
    draw();
  });

  // Container resizes (window drag, sidebar changes…) need a redraw too.
  $effect(() => {
    const el = canvas;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(() => draw());
    ro.observe(el.parentElement ?? el);
    return () => ro.disconnect();
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
  .wrap canvas { display: block; width: 100%; }
  .empty { text-align: center; color: var(--color-text-muted); font-size: 13px; margin-top: 8px; }
</style>
