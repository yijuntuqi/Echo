<script lang="ts">
  import { evolutionStore } from '$lib/stores/evolution';

  const STAGES = ['egg', 'child', 'teen', 'adult', 'ultimate'] as const;
  const COLORS: Record<string, string> = {
    egg: '#fff8dc', child: '#ffe4b5', teen: '#ffd700', adult: '#ff8c00', ultimate: '#ff4500',
  };
  const LABELS: Record<string, string> = {
    egg: '蛋', child: '幼年', teen: '少年', adult: '成年', ultimate: '究极',
  };

  let canvas: HTMLCanvasElement | undefined = $state();

  const currentIndex = $derived(STAGES.indexOf(evolutionStore.stage as (typeof STAGES)[number]));
  const personality = $derived(evolutionStore.personality);

  function draw() {
    const ctx = canvas?.getContext('2d');
    if (!ctx || !canvas) return;

    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, w, h);

    const cx = w / 2;
    const cy = h / 2;
    const ring = Math.min(w, h) * 0.34;

    // Outer ring: the five stages, connected in order.
    STAGES.forEach((stage, i) => {
      const a = -Math.PI / 2 + (i / STAGES.length) * Math.PI * 2;
      const x = cx + Math.cos(a) * ring;
      const y = cy + Math.sin(a) * ring;
      const isCurrent = i === currentIndex;
      const isPast = i < currentIndex;

      if (i < STAGES.length - 1) {
        const na = -Math.PI / 2 + ((i + 1) / STAGES.length) * Math.PI * 2;
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(cx + Math.cos(na) * ring, cy + Math.sin(na) * ring);
        ctx.strokeStyle = isPast ? COLORS[stage] : 'rgba(0,0,0,0.10)';
        ctx.lineWidth = 3;
        ctx.stroke();
      }

      ctx.beginPath();
      ctx.arc(x, y, isCurrent ? 26 : 19, 0, Math.PI * 2);
      ctx.fillStyle = isPast || isCurrent ? COLORS[stage] : 'rgba(0,0,0,0.06)';
      ctx.fill();
      ctx.strokeStyle = isCurrent ? 'rgba(0,0,0,0.45)' : 'rgba(0,0,0,0.12)';
      ctx.lineWidth = isCurrent ? 2.5 : 1;
      ctx.stroke();

      if (isCurrent) {
        ctx.beginPath();
        ctx.arc(x, y, 32, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * evolutionStore.progress);
        ctx.strokeStyle = COLORS[stage];
        ctx.lineWidth = 4;
        ctx.lineCap = 'round';
        ctx.stroke();
      }

      ctx.fillStyle = isPast || isCurrent ? '#333' : 'rgba(0,0,0,0.35)';
      ctx.font = `${isCurrent ? 12 : 10}px system-ui, sans-serif`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(LABELS[stage], x, y);
    });

    // Inner radar: first 8 personality dimensions.
    if (personality) drawRadar(ctx, cx, cy, ring * 0.55);
  }

  function drawRadar(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number) {
    const dims = 8;
    ctx.beginPath();
    for (let i = 0; i < dims; i++) {
      const a = (i / dims) * Math.PI * 2 - Math.PI / 2;
      const v = ((personality![i] ?? 0) + 1) / 2; // map [-1,1] -> [0,1]
      const x = cx + Math.cos(a) * r * v;
      const y = cy + Math.sin(a) * r * v;
      i === 0 ? ctx.moveTo(x, y) : ctx.lineTo(x, y);
    }
    ctx.closePath();
    ctx.fillStyle = 'rgba(255, 107, 53, 0.18)';
    ctx.fill();
    ctx.strokeStyle = 'rgba(255, 107, 53, 0.7)';
    ctx.lineWidth = 1.5;
    ctx.stroke();
  }

  $effect(() => {
    // Track the reactive inputs the drawing depends on.
    void evolutionStore.stage;
    void evolutionStore.progress;
    void evolutionStore.personality;
    draw();
  });
</script>

<canvas bind:this={canvas} class="tree" aria-label="进化树"></canvas>

<style>
  .tree { display: block; width: 100%; max-width: 380px; height: auto; margin: 0 auto; aspect-ratio: 1; }
</style>
