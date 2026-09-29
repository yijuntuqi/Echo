<!-- EvolutionTree - Canvas visualization -->
<script lang="ts">
    import { evolutionStore } from '$lib/stores/evolution';
    import { onMount } from 'svelte';
    
    let canvas: HTMLCanvasElement;
    const STAGE_ORDER = ['egg', 'child', 'teen', 'adult', 'ultimate'] as const;
    const STAGE_COLORS: Record<string, string> = {
        egg: '#fff8dc', child: '#ffe4b5', teen: '#ffd700', adult: '#ff8c00', ultimate: '#ff4500'
    };
    
    function draw() {
        if (!canvas) return;
        const ctx = canvas.getContext('2d')!;
        const dpr = window.devicePixelRatio || 1;
        const w = canvas.clientWidth, h = canvas.clientHeight;
        canvas.width = w * dpr; canvas.height = h * dpr;
        ctx.scale(dpr, dpr);
        ctx.clearRect(0, 0, w, h);
        
        const cx = w / 2, cy = h / 2;
        const radius = Math.min(w, h) * 0.35;
        
        STAGE_ORDER.forEach((stage, i) => {
            const angle = -Math.PI / 2 + (i / STAGE_ORDER.length) * Math.PI * 2;
            const x = cx + Math.cos(angle) * radius;
            const y = cy + Math.sin(angle) * radius;
            const isCurrent = stage === evolutionStore.currentStage;
            const isPast = STAGE_ORDER.indexOf(stage) < STAGE_ORDER.indexOf(evolutionStore.currentStage);
            
            if (i < STAGE_ORDER.length - 1) {
                const nextAngle = -Math.PI / 2 + ((i + 1) / STAGE_ORDER.length) * Math.PI * 2;
                const nx = cx + Math.cos(nextAngle) * radius;
                const ny = cy + Math.sin(nextAngle) * radius;
                ctx.beginPath();
                ctx.moveTo(x, y);
                ctx.lineTo(nx, ny);
                ctx.strokeStyle = isPast ? STAGE_COLORS[stage] : '#ddd';
                ctx.lineWidth = 3;
                ctx.stroke();
            }
            
            ctx.beginPath();
            ctx.arc(x, y, isCurrent ? 28 : 20, 0, Math.PI * 2);
            ctx.fillStyle = isPast || isCurrent ? STAGE_COLORS[stage] : '#eee';
            ctx.fill();
            ctx.strokeStyle = isCurrent ? '#333' : '#ccc';
            ctx.lineWidth = isCurrent ? 3 : 1;
            ctx.stroke();
            
            if (isCurrent) {
                ctx.beginPath();
                ctx.arc(x, y, 34, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * evolutionStore.progress);
                ctx.strokeStyle = STAGE_COLORS[stage];
                ctx.lineWidth = 4;
                ctx.lineCap = 'round';
                ctx.stroke();
            }
            
            ctx.fillStyle = isPast || isCurrent ? '#fff' : '#999';
            ctx.font = `bold ${isCurrent ? 16 : 12}px sans-serif`;
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillText(stage.charAt(0).toUpperCase(), x, y);
        });
        
        if (evolutionStore.personality) {
            drawPersonalityRadar(ctx, cx, cy, radius * 0.6);
        }
    }
    
    function drawPersonalityRadar(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number) {
        const vec = evolutionStore.personality!;
        const dims = 8;
        ctx.beginPath();
        for (let i = 0; i < dims; i++) {
            const angle = (i / dims) * Math.PI * 2 - Math.PI / 2;
            const val = (vec[i] + 1) / 2;
            const x = cx + Math.cos(angle) * r * val;
            const y = cy + Math.sin(angle) * r * val;
            if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
        }
        ctx.closePath();
        ctx.fillStyle = 'rgba(255, 100, 100, 0.15)';
        ctx.fill();
        ctx.strokeStyle = 'rgba(255, 100, 100, 0.6)';
        ctx.lineWidth = 1;
        ctx.stroke();
    }
    
    $effect(() => { draw(); });
</script>

<canvas 
    bind:this={canvas} 
    class="evolution-canvas" 
    width={400} height={400}
    style="width: 100%; height: auto; max-width: 400px;"
/>

<style>
    .evolution-canvas { display: block; margin: 0 auto; }
</style>