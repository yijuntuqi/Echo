<!-- PetWindow: root component for the transparent always-on-top pet window -->
<script lang="ts">
  import { onMount } from 'svelte';
  import PetAvatar from '$lib/components/pet/PetAvatar.svelte';
  import { petStore } from '$lib/stores/pet';
  import { getPetPosition, showPetOverlay } from '$lib/api/commands';
  import { on } from '$lib/api/events';

  onMount(() => {
    // Keep the teardown synchronous; do the async work inside.
    let disposed = false;
    let unlistenEvolution: (() => void) | undefined;

    void (async () => {
      // Restore last known position, then reveal the window.
      try {
        const pos = await getPetPosition();
        if (!disposed && pos) petStore.setPosition({ x: pos.x, y: pos.y });
      } catch {
        /* first run: no saved position yet */
      }
      try {
        await showPetOverlay();
      } catch {
        /* window may already be visible */
      }

      // This window owns the avatar, so it — not the main window — plays the
      // evolve animation when the backend pushes a transition.
      const unlisten = await on('evolution:triggered', (e) => {
        petStore.setStage(e.to_stage);
      });
      if (disposed) unlisten();
      else unlistenEvolution = unlisten;
    })();

    return () => {
      disposed = true;
      unlistenEvolution?.();
    };
  });
</script>

<div class="pet-window">
  <PetAvatar />
</div>

<style>
  /* The window itself is transparent; nothing here may paint a background.
     global.css paints `body` for the main window (Vite injects it after this
     page's inline styles), so the pet window re-clears it here. */
  :global(html),
  :global(body) {
    background: transparent !important;
  }
  .pet-window { width: 100%; height: 100%; background: transparent; overflow: hidden; }
</style>
