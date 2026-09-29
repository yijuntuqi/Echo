<!-- PetOverlay - Transparent window root component -->
<script lang="ts">
    import { petStore } from '$lib/stores/pet';
    import PetAvatar from './PetAvatar.svelte';
    import { onMount } from 'svelte';
    import { setPetPosition, getPetPosition } from '$lib/api/commands';
    
    onMount(async () => {
        const pos = await getPetPosition();
        if (pos) petStore.setPosition({ x: pos.x, y: pos.y });
    });
    
    function handlePositionChange(pos: { x: number; y: number }) {
        setPetPosition(pos.x, pos.y);
    }
</script>

{#if $petStore.isVisible}
    <PetAvatar 
        bind:position={$petStore.position}
        on:positionChange={handlePositionChange}
    />
{/if}