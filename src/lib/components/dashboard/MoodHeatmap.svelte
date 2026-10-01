<script lang="ts">
  import { onDestroy } from 'svelte';
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
  // Hover pop animation: the cell grows with a springy overshoot.
  const POP_MS = 220;

  const COLORS: Record<string, string> = {
    happy: '#2ecc71', sad: '#3498db', anxious: '#f39c12', calm: '#1abc9c',
    angry: '#e74c3c', excited: '#e67e22', bored: '#95a5a6', lonely: '#9b59b6',
    grateful: '#f1c40f', neutral: '#bdc3c7',
  };
  const EMOTION_LABELS: Record<string, string> = {
    happy: '开心', sad: '难过', anxious: '焦虑', calm: '平静', angry: '生气',
    excited: '兴奋', bored: '无聊', lonely: '孤独', grateful: '感激', neutral: '平静',
  };

  let canvas: HTMLCanvasElement | undefined = $state();
  // Hovered grid cell; drives both the pop animation and the tooltip.
  let hover = $state<{ week: number; dow: number } | null>(null);
  let popStart = 0;
  let rafId = 0;

  /** Local-time YYYY-MM-DD. Mood rows are stored with localtime dates, and
   * toISOString() would render UTC — one day off in UTC+8 afternoons. */
  function localIso(d: Date): string {
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${d.getFullYear()}-${m}-${day}`;
  }

  /** Sunday that starts the fixed 16-week window ending with this week. */
  function gridStart(): Date {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const weekEnd = new Date(today);
    weekEnd.setDate(weekEnd.getDate() + (6 - weekEnd.getDay()));
    const start = new Date(weekEnd);
    start.setDate(start.getDate() - (WEEKS * 7 - 1));
    return start;
  }

  /** Ease-out-back: overshoots slightly past the target, then settles —
   * at t=0 it is 0, peaks above 1 around t≈0.7, lands exactly on 1. */
  function popEase(t: number): number {
    const c1 = 1.70158;
    const c3 = c1 + 1;
    return 1 + c3 * (t - 1) ** 3 + c1 * (t - 1) ** 2;
  }

  /** Replay the pop animation for the newly hovered cell. */
  function animatePop(): void {
    cancelAnimationFrame(rafId);
    popStart = performance.now();
    const tick = () => {
      draw();
      if (performance.now() - popStart < POP_MS) rafId = requestAnimationFrame(tick);
    };
    rafId = requestAnimationFrame(tick);
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

    const start = gridStart();
    const today = new Date();
    today.setHours(0, 0, 0, 0);
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

    // Cells: emotion colour shaded by weight; faint grey when empty. The
    // hovered cell grows with a springy pop and lifts on a soft shadow.
    for (let week = 0; week < WEEKS; week++) {
      for (let dow = 0; dow < 7; dow++) {
        const day = new Date(start.getTime() + (week * 7 + dow) * 864e5);
        if (day.getTime() > Date.now()) break;
        const x = LABEL_W + week * STEP;
        const y = LABEL_H + dow * STEP;
        const entry: MoodEntry | undefined = byDay.get(localIso(day));

        const isHover = hover !== null && hover.week === week && hover.dow === dow;
        const grow = isHover
          ? 0.45 * popEase(Math.min(1, (performance.now() - popStart) / POP_MS))
          : 0;
        const pad = (grow * CELL) / 2;

        if (isHover) {
          ctx.save();
          ctx.shadowColor = 'rgba(0, 0, 0, 0.25)';
          ctx.shadowBlur = 6;
          ctx.shadowOffsetY = 2;
        }
        if (entry) {
          ctx.fillStyle = COLORS[entry.emotion] ?? COLORS.neutral;
          ctx.globalAlpha = Math.min(1, Math.max(0.3, entry.weight));
        } else {
          ctx.fillStyle = COLORS.neutral;
          ctx.globalAlpha = isHover ? 0.3 : 0.12;
        }
        ctx.beginPath();
        ctx.roundRect(x - pad, y - pad, CELL + grow * CELL, CELL + grow * CELL, 3 + pad * 0.6);
        ctx.fill();
        if (isHover) ctx.restore();

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

    // Tooltip for the hovered day, drawn last so it overlays everything.
    if (hover) {
      const day = new Date(start.getTime() + (hover.week * 7 + hover.dow) * 864e5);
      const entry: MoodEntry | undefined = byDay.get(localIso(day));
      const lines = entry
        ? [
            `${day.getMonth() + 1}月${day.getDate()}日 · ${EMOTION_LABELS[entry.emotion] ?? entry.emotion}`,
            `强度 ${Math.round(Math.min(1, Math.max(0, entry.weight)) * 100)}%`,
          ]
        : [`${day.getMonth() + 1}月${day.getDate()}日`, '没有记录'];

      ctx.font = '11px system-ui, sans-serif';
      ctx.textBaseline = 'top';
      const boxW = Math.max(...lines.map((l) => ctx.measureText(l).width)) + 16;
      const boxH = lines.length * 15 + 10;
      const cx = LABEL_W + hover.week * STEP + CELL / 2;
      const cy = LABEL_H + hover.dow * STEP;
      let bx = Math.min(Math.max(2, cx - boxW / 2), w - boxW - 2);
      let by = cy - boxH - 6;
      if (by < 2) by = cy + CELL + 6;

      ctx.fillStyle = 'rgba(30, 30, 34, 0.92)';
      ctx.beginPath();
      ctx.roundRect(bx, by, boxW, boxH, 5);
      ctx.fill();
      ctx.fillStyle = '#fff';
      lines.forEach((line, i) => {
        ctx.fillText(line, bx + 8, by + 6 + i * 15);
      });
    }
  }

  /** Hit-test the grid under the cursor; null when off-grid or in the future. */
  function onMove(e: MouseEvent): void {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const gx = e.clientX - rect.left - LABEL_W;
    const gy = e.clientY - rect.top - LABEL_H;
    const week = Math.floor(gx / STEP);
    const dow = Math.floor(gy / STEP);
    const valid =
      gx >= 0 &&
      gy >= 0 &&
      week >= 0 &&
      week < WEEKS &&
      dow >= 0 &&
      dow < 7 &&
      gridStart().getTime() + (week * 7 + dow) * 864e5 <= Date.now();
    const next = valid ? { week, dow } : null;
    const changed =
      (!next && hover !== null) ||
      (next !== null && (hover === null || hover.week !== next.week || hover.dow !== next.dow));
    if (changed && next) animatePop();
    hover = next;
    if (canvas) canvas.style.cursor = next ? 'pointer' : 'default';
    if (!next) draw();
  }

  function onLeave(): void {
    cancelAnimationFrame(rafId);
    hover = null;
    if (canvas) canvas.style.cursor = 'default';
    draw();
  }

  onDestroy(() => cancelAnimationFrame(rafId));

  $effect(() => {
    void moodStore.entries;
    void hover;
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
  <canvas
    bind:this={canvas}
    aria-label="心情热力图"
    onmousemove={onMove}
    onmouseleave={onLeave}
  ></canvas>
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
