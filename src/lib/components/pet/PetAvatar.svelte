<!-- PetAvatar - SVG state machine animation -->
<script lang="ts">
    import { petStore } from '$lib/stores/pet';
    import { onMount, createEventDispatcher } from 'svelte';
    import { motion, spring } from 'svelte-motion';
    import type { PetAnimationState, Vec2 } from '$lib/api/types';
    
    export let position = $bindable($petStore.position);
    
    const dispatch = createEventDispatcher<{ openChat: void; contextMenu: { x: number; y: number } }>();
    
    const variants = {
        idle: { opacity: 1, scale: 1, rotate: 0 },
        walk: { x: [0, -5, 5, -5, 0], transition: { duration: 2, repeat: Infinity } },
        sleep: { y: [0, -3, 0], opacity: [1, 0.7, 1], transition: { duration: 3, repeat: Infinity } },
        talk: { scale: [1, 1.05, 1], y: [0, -2, 0], transition: { duration: 0.3, repeat: Infinity } },
        react: { scale: [1, 1.2, 1], rotate: [0, -10, 10, 0], transition: { duration: 0.5 } },
        evolve: { scale: [1, 0, 1.5, 1], rotate: [0, 360], opacity: [1, 0, 1], transition: { duration: 1.5, ease: 'easeOut' } },
    };
    
    let currentSvg = $derived(petStore.svgPaths[petStore.animation] ?? petStore.svgPaths.idle);
    let svgContent = $state<string>('');
    const svgCache = new Map<string, string>();
    
    async function preloadSvgs() {
        for (const path of Object.values(petStore.svgPaths)) {
            try {
                const res = await fetch(path);
                if (res.ok) svgCache.set(path, await res.text());
            } catch {}
        }
    }
    
    $effect(() => {
        if (svgCache.has(currentSvg)) {
            svgContent = svgCache.get(currentSvg)!;
        } else {
            svgContent = `<svg viewBox="0 0 200 200"><circle cx="100" cy="100" r="80" fill="#ffd700"/></svg>`;
        }
    });
    
    onMount(preloadSvgs);
    
    let dragStart = { x: 0, y: 0 };
    function handleMouseDown(e: MouseEvent) {
        if (petStore.clickThrough) return;
        dragStart = { x: e.clientX - position.x, y: e.clientY - position.y };
        window.addEventListener('mousemove', handleMouseMove);
        window.addEventListener('mouseup', handleMouseUp);
        petStore.playAnimation('walk');
    }
    function handleMouseMove(e: MouseEvent) {
        const newPos = { x: e.clientX - dragStart.x, y: e.clientY - dragStart.y };
        position = newPos;
    }
    function handleMouseUp() {
        window.removeEventListener('mousemove', handleMouseMove);
        window.removeEventListener('mouseup', handleMouseUp);
        petStore.playAnimation('idle');
    }
    
    function handleClick() {
        petStore.playAnimation('react');
        petStore.touch();
        dispatch('openChat');
    }
    
    function handleContextMenu(e: MouseEvent) {
        e.preventDefault();
        dispatch('contextMenu', { x: e.clientX, y: e.clientY });
    }
</script>

<div 
    class="pet-avatar"
    style:transform="translate({position.x}px, {position.y}px)"
    on:mousedown={handleMouseDown}
    on:click={handleClick}
    on:contextmenu={handleContextMenu}
>
    <motion.div
        animate={variants[petStore.animation] || variants.idle}
        transition={{ type: 'spring', stiffness: 300, damping: 30 }}
    >
        {@html svgContent}
    </motion.div>
    
    {#if petStore.animation === 'evolve'}
        <div class="evolve-ring" />
    {/if}
</div>

<style>
    .pet-avatar {
        position: fixed;
        pointer-events: auto;
        z-index: 9999;
        width: 160px;
        height: 160px;
        cursor: grab;
        user-select: none;
        -webkit-user-select: none;
    }
    .pet-avatar:active { cursor: grabbing; }
    .evolve-ring {
        position: absolute;
        inset: -20px;
        border: 3px solid #ffd700;
        border-radius: 50%;
        animation: pulse 1.5s ease-out forwards;
    }
    @keyframes pulse { from { opacity: 1; transform: scale(0.8); } to { opacity: 0; transform: scale(1.5); } }
</style>