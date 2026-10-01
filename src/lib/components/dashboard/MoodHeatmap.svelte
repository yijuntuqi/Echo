<script lang="ts">
  import type { MoodEntry } from '$lib/api/types';
  import { moodStore } from '$lib/stores/mood';

  const CELL = 12;
  const GAP = 3;
  const STEP = CELL + GAP;
  // A fixed ~4-month window, GitHub-style: empty weeks render as faint cells
  // instead of the grid starting at the first record (which made day-one data
  // look like a lone dot).
  const WEEKS = 16;
  const LABEL_W = 20; // left gutter for weekday labels
  const LABEL_H = 16; // top gutter for month labels
  const LEGEND_H = 24; // bottom strip for the intensity legend

  const COLORS: Record<string, string> = {
    happy: '#2ecc71', sad: '#3498db', anxious: '#f39c12', calm: '#1abc9c',
    angry: '#e74c3c', excited: '#e67e22', bored: '#95a5a6', lonely: '#9b59b6',
    grateful: '#f1c40f', neutral: '#bdc3c7',
  };

  let canvas: HTMLCanvasElement | undefined = $state();

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
    const gridW = WEEKS * STEP;
    const gridH = 7 * STEP - GAP;
    const w = LABEL_W + gridW;
    const h = LABEL_H + gridH + LEGEND_H;

    canvas.style.width = `${w}px`;
    canvas.style.height = `${h}px`;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);

    // Grid origin: the Sunday 16 weeks before this week's end.
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const weekEnd = new Date(today);
    weekEnd.setDate(weekEnd.getDate() + (6 - weekEnd.getDay()));
    const start = new Date(weekEnd);
    start.setDate(start.getDate() - (WEEKS * 7 - 1));

    const byDay = new Map(entries.map((e) => [e.date, e]));
    const muted =
      getComputedStyle(document.documentElement).getPropertyValue('--color-text-muted').trim() ||
      '#999';

    ctx.font = '10px system-ui, sans-serif';
    ctx.textBaseline = 'top';

    // Month labels above the first column of each new month.
    let lastMonth = -1;
    for (let week = 0; week < WEEKS; week++) {
      const col = new Date(start.getTime() + week * 7 * 864e5);
      if (col.getMonth() !== lastMonth) {
        lastMonth = col.getMonth();
        ctx.fillStyle = muted;
        ctx.fillText(`${lastMonth + 1}月`, LABEL_W + week * STEP, 2);
      }
    }

    // Weekday labels on the Mon / Wed / Fri rows.
    ctx.fillStyle = muted;
    (['一', '三', '五'] as const).forEach((label, i) => {
      ctx.fillText(label, 0, LABEL_H + (1 + i * 2) * STEP + 1);
    });

    // Cells: emotion colour shaded by weight; faint grey when empty.
    for (let week = 0; week < WEEKS; week++) {
      for (let dow = 0; dow < 7; dow++) {
        const day = new Date(start.getTime() + (week * 7 + dow) * 864e5);
        if (day.getTime() > Date.now()) break;
        const x = LABEL_W + week * STEP;
        const y = LABEL_H + dow * STEP;
        const entry: MoodEntry | undefined = byDay.get(localIso(day));

        if (entry) {
          ctx.fillStyle = COLORS[entry.emotion] ?? COLORS.neutral;
          ctx.globalAlpha = Math.min(1, Math.max(0.3, entry.weight));
        } else {
          ctx.fillStyle = COLORS.neutral;
          ctx.globalAlpha = 0.12;
        }
        ctx.beginPath();
        ctx.roundRect(x, y, CELL, CELL, 3);
        ctx.fill();

        // Today gets an outline so the grid reads as "up to now".
        if (day.getTime() === today.getTime()) {
          ctx.globalAlpha = 1;
          ctx.strokeStyle = muted;
          ctx.lineWidth = 1.5;
          ctx.strokeRect(x - 1.5, y - 1.5, CELL + 3, CELL + 3);
        }
      }
    }
    ctx.globalAlpha = 1;

    // Intensity legend: five steps from faint to solid.
    const legendY = LABEL_H + gridH + 8;
    ctx.fillStyle = muted;
    ctx.textBaseline = 'middle';
    ctx.fillText('低', LABEL_W, legendY + CELL / 2);
    for (let i = 0; i < 5; i++) {
      ctx.fillStyle = COLORS.neutral;
      ctx.globalAlpha = 0.15 + i * 0.2125;
      ctx.beginPath();
      ctx.roundRect(LABEL_W + 16 + i * STEP, legendY, CELL, CELL, 3);
      ctx.fill();
    }
    ctx.globalAlpha = 1;
    ctx.fillStyle = muted;
    ctx.fillText('高', LABEL_W + 16 + 5 * STEP, legendY + CELL / 2);
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
    <p class="empty">还没有心情记录，和 Echo 聊聊天吧</p>
  {/if}
</div>

<style>
  .wrap {
    width: 100%;
    overflow-x: auto;
    text-align: center;
  }
  .wrap canvas { display: inline-block; }
  .empty { text-align: center; color: var(--color-text-muted); font-size: 13px; margin-top: 8px; }
</style>
