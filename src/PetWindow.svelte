<!-- PetWindow: root component for the transparent always-on-top pet window -->
<script lang="ts">
  import { onMount } from 'svelte';
  import PetAvatar from '$lib/components/pet/PetAvatar.svelte';
  import { petStore } from '$lib/stores/pet';
  import { getPetPosition, showPetOverlay } from '$lib/api/commands';

  onMount(async () => {
    // Restore last known position, then reveal the window.
    try {
      const pos = await getPetPosition();
      if (pos) petStore.setPosition({ x: pos.x, y: pos.y });
    } catch {
      /* first run: no saved position yet */
    }
    try {
      await showPetOverlay();
    } catch {
      /* window may already be visible */
    }
  });
</script>

<div class="pet-window">
  <PetAvatar />
</div>

<style>
  /* The window itself is transparent; nothing here may paint a background. */
  .pet-window { width: 100%; height: 100%; background: transparent; overflow: hidden; }
</style>
