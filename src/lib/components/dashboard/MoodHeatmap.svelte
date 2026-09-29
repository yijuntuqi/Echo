<!-- MoodHeatmap - GitHub contribution style heatmap -->
<script lang="ts">
    import { moodStore } from '$lib/stores/mood';
    import { onMount } from 'svelte';
    
    let canvas: HTMLCanvasElement;
    const CELL = 14, GAP = 2;
    const EMOTION_COLORS: Record<string, string> = {
        happy: '#2ecc71', sad: '#3498db', anxious: '#f39c12', calm: '#1abc9c',
        angry: '#e74c3c', excited: '#e67e22', bored: '#95a5a6', lonely: '#9b59b6',
        grateful: '#f1c40f', neutral: '#bdc3c7'
    };
    
    function draw() {
        if (!canvas) return;
        const ctx = canvas.getContext('2d')!;
        const dpr = window.devicePixelRatio || 1;
        const w = canvas.clientWidth, h = canvas.clientHeight;
        canvas.width = w * dpr; canvas.height = h * dpr;
        ctx.scale(dpr, dpr);
        ctx.clearRect(0, 0, w, h);
        
        const today = new Date();
        const start = new Date(today.getFullYear(), 0, 1);
        
        moodStore.entries.forEach(entry => {
            const date = new Date(entry.date);
            const dayOfWeek = date.getDay();
            const weekOfYear = Math.floor((date.getTime() - start.getTime()) / (7 * 864e5));
            const x = weekOfYear * (CELL + GAP);
            const y = dayOfWeek * (CELL + GAP);
            
            ctx.fillStyle = EMOTION_COLORS[entry.emotion] ?? EMOTION_COLORS.neutral;
            ctx.globalAlpha = Math.min(1, entry.weight);
            ctx.fillRect(x, y, CELL, CELL);
            ctx.globalAlpha = 1;
        });
    }
    
    $effect(() => { draw(); });
</script>

<canvas bind:this={canvas} class="heatmap-canvas" />

<style>
    .heatmap-canvas { display: block; width: 100%; max-width: 700px; margin: 0 auto; }
</style>