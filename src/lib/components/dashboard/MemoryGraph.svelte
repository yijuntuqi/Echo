<!-- MemoryGraph - Semantic memory visualization -->
<script lang="ts">
    import { onMount } from 'svelte';
    import type { VectorHit } from '$lib/api/types';
    
    export let hits: VectorHit[] = [];
    let canvas: HTMLCanvasElement;
    
    function draw() {
        if (!canvas || hits.length === 0) return;
        const ctx = canvas.getContext('2d')!;
        const dpr = window.devicePixelRatio || 1;
        const w = canvas.clientWidth, h = canvas.clientHeight;
        canvas.width = w * dpr; canvas.height = h * dpr;
        ctx.scale(dpr, dpr);
        ctx.clearRect(0, 0, w, h);
        
        const cx = w / 2, cy = h / 2;
        const maxR = Math.min(w, h) * 0.4;
        
        hits.forEach((hit, i) => {
            const angle = (i / hits.length) * Math.PI * 2;
            const dist = 1 - hit.distance / 2; // 0~1
            const r = maxR * dist * 0.8;
            const x = cx + Math.cos(angle) * r;
            const y = cy + Math.sin(angle) * r;
            const size = 8 + dist * 16;
            
            ctx.beginPath();
            ctx.arc(x, y, size, 0, Math.PI * 2);
            ctx.fillStyle = hit.memory_type === 'conversation' ? '#ff6b35' : '#3498db';
            ctx.globalAlpha = 0.7;
            ctx.fill();
            ctx.globalAlpha = 1;
            
            // Line to center
            ctx.beginPath();
            ctx.moveTo(cx, cy);
            ctx.lineTo(x, y);
            ctx.strokeStyle = 'rgba(255,107,53,0.3)';
            ctx.lineWidth = 1;
            ctx.stroke();
        });
        
        // Center
        ctx.beginPath();
        ctx.arc(cx, cy, 6, 0, Math.PI * 2);
        ctx.fillStyle = '#fff';
        ctx.fill();
    }
    
    $effect(() => { draw(); });
</script>

<canvas bind:this={canvas} class="memory-canvas" width={300} height={300} style="width: 100%; max-width: 300px;" />

<style>
    .memory-canvas { display: block; margin: 0 auto; }
</style>