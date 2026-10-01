<!-- PetWindow: transparent always-on-top pet window; hosts the pet and,
     when open, the chat panel docked beside it. Clicking the pet expands
     the window around it; closing the panel shrinks it back. -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow, PhysicalPosition, PhysicalSize, currentMonitor } from '@tauri-apps/api/window';
  import PetAvatar from '$lib/components/pet/PetAvatar.svelte';
  import ChatPanel from '$lib/components/chat/ChatPanel.svelte';
  import { petStore } from '$lib/stores/pet';
  import { getPetPosition, showPetOverlay, setPetWindowShape, placePetWindow } from '$lib/api/commands';
  import { on } from '$lib/api/events';

  // Layout constants in CSS pixels. The pet keeps its 200x200 square; the
  // chat panel docks beside it inside the same (transparent) window.
  const PET_EDGE = 200;
  const PANEL_W = 384;
  const WINDOW_W = PET_EDGE + PANEL_W + 8; // 592
  const WINDOW_H = 560;

  let panelOpen = $state(false);
  let expanded = $state(false);
  // Where the pet sits inside the expanded window, and where the panel starts.
  let petX = $state(0);
  let petY = $state(0);
  let panelX = $state(PET_EDGE + 4);

  onMount(() => {
    // Keep the teardown synchronous; do the async work inside.
    let disposed = false;
    let unlistenEvolution: (() => void) | undefined;
    let unlistenChatOpen: (() => void) | undefined;

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
      // The collapsed window is a 200x200 square whose transparent corners
      // would block clicks on windows underneath (e.g. the main window's
      // navigation): trim it to the pet's egg silhouette from the start.
      try {
        const scale = window.devicePixelRatio || 1;
        await setPetWindowShape({
          ellipse: [
            Math.round(28 * scale),
            Math.round(8 * scale),
            Math.round(144 * scale),
            Math.round(176 * scale),
          ],
        });
      } catch {
        /* non-Tauri context or unsupported: window stays square */
      }

      // This window owns the avatar, so it — not the main window — plays the
      // evolve animation when the backend pushes a transition.
      try {
        const unlisten = await on('evolution:triggered', (e) => {
          petStore.setStage(e.to_stage);
        });
        if (disposed) unlisten();
        else unlistenEvolution = unlisten;
      } catch (e) {
        console.warn('evolution:triggered listener unavailable:', e);
      }

      // Clicking the pet raises `chat:open` (backend broadcast). The panel
      // rides in this window, so expansion/collapse happens right here.
      try {
        const unlisten = await on('chat:open', () => {
          if (panelOpen) void closePanel();
          else void openPanel();
        });
        if (disposed) unlisten();
        else unlistenChatOpen = unlisten;
      } catch (e) {
        console.warn('chat:open listener unavailable:', e);
      }
    })();

    return () => {
      disposed = true;
      unlistenEvolution?.();
      unlistenChatOpen?.();
    };
  });

  // Closing the panel from inside (the × button) unbinds `panelOpen` to
  // false; shrink the window back so no invisible rectangle blocks clicks.
  $effect(() => {
    if (!panelOpen && expanded) void closePanel();
  });

  /** Expand the window around the pet and dock the panel beside it. */
  async function openPanel(): Promise<void> {
    if (expanded) {
      panelOpen = true;
      return;
    }
    try {
      const win = getCurrentWindow();
      const scale = window.devicePixelRatio || 1;
      const w = Math.round(WINDOW_W * scale);
      const h = Math.round(WINDOW_H * scale);
      const pos = await win.outerPosition();

      let x = pos.x;
      let y = pos.y;
      // The pet anchors to one corner of the expanded window; the window
      // always grows right/down from its origin, so when screen space runs
      // out we shift the origin and anchor the pet to the opposite edge —
      // the pet itself never moves on screen.
      let expandLeft = false;
      let expandUp = false;
      try {
        const mon = await currentMonitor();
        if (mon) {
          if (pos.x + w > mon.position.x + mon.size.width) {
            expandLeft = true;
            x = pos.x - (w - Math.round(PET_EDGE * scale));
          }
          if (pos.y + h > mon.position.y + mon.size.height) {
            expandUp = true;
            y = pos.y - (h - Math.round(PET_EDGE * scale));
          }
        }
      } catch {
        /* monitor info unavailable: expand right/down from the origin */
      }

      petX = expandLeft ? WINDOW_W - PET_EDGE : 0;
      petY = expandUp ? WINDOW_H - PET_EDGE : 0;
      panelX = expandLeft ? 4 : PET_EDGE + 4;

      if (x !== pos.x || y !== pos.y) {
        // Atomic move+resize: separate calls can paint the pet outside the
        // not-yet-grown window for a frame.
        await placePetWindow(x, y, w, h).catch(() => {
          /* fall back below if the atomic call is unavailable */
          void win.setPosition(new PhysicalPosition(x, y));
          void win.setSize(new PhysicalSize(w, h));
        });
      } else {
        await win.setSize(new PhysicalSize(w, h));
      }
      // The grown window is mostly empty transparency: without a hit-test
      // region that invisible rectangle would swallow clicks meant for
      // windows underneath (e.g. the main window's navigation).
      await setPetWindowShape({
        rects: [
          [Math.round(petX * scale), Math.round(petY * scale), Math.round(PET_EDGE * scale), Math.round(PET_EDGE * scale)],
          [Math.round(panelX * scale), 0, Math.round(PANEL_W * scale), Math.round(WINDOW_H * scale)],
        ],
      }).catch(() => {});
      expanded = true;
      panelOpen = true;
    } catch {
      // Not running inside Tauri (plain browser tab): just show the panel.
      expanded = true;
      panelOpen = true;
    }
  }

  /** Shrink back to just the pet's square, anchored where the pet is now.
   * The pet itself never hides or jumps: it only stops sharing the window
   * with the panel. Hiding the pet is a tray-only action. */
  async function closePanel(): Promise<void> {
    panelOpen = false;
    if (!expanded) return;
    expanded = false;
    try {
      const win = getCurrentWindow();
      const scale = window.devicePixelRatio || 1;
      // The window may have been dragged while expanded, so anchor the
      // shrink to wherever the pet actually is right now (window origin +
      // pet offset) instead of a stale pre-expand position.
      const pos = await win.outerPosition();
      const x = pos.x + Math.round(petX * scale);
      const y = pos.y + Math.round(petY * scale);
      // Reset the anchor and re-place the window in the same tick: petX must
      // reach 0 as the 200x200 clip applies, or the pet would be drawn
      // outside the shrunk window and vanish for a frame.
      petX = 0;
      petY = 0;
      panelX = PET_EDGE + 4;
      await placePetWindow(
        x,
        y,
        Math.round(PET_EDGE * scale),
        Math.round(PET_EDGE * scale),
      ).catch(() => {
        void win.setSize(new PhysicalSize(Math.round(PET_EDGE * scale), Math.round(PET_EDGE * scale)));
        void win.setPosition(new PhysicalPosition(x, y));
      });
      // Even collapsed, the square's transparent corners would block clicks
      // on windows underneath: keep a hit-test region shaped as the pet's
      // egg silhouette so only the pet itself is interactive.
      await setPetWindowShape({
        ellipse: [
          Math.round(28 * scale),
          Math.round(8 * scale),
          Math.round(144 * scale),
          Math.round(176 * scale),
        ],
      }).catch(() => {});
    } catch {
      /* ignore */
    }
  }
</script>

<div class="pet-window">
  <div class="pet-anchor" style:left="{petX}px" style:top="{petY}px">
    <PetAvatar onopenchat={() => void openPanel()} />
  </div>

  {#if panelOpen}
    <div class="panel-slot" style:left="{panelX}px">
      <ChatPanel bind:open={panelOpen} docked />
    </div>
  {/if}
</div>

<style>
  /* The window itself is transparent; nothing here may paint a background.
     global.css paints `body` for the main window (Vite injects it after this
     page's inline styles), so the pet window re-clears it here. */
  :global(html),
  :global(body) {
    background: transparent !important;
  }
  .pet-window { position: absolute; inset: 0; background: transparent; overflow: hidden; }
  /* The pet keeps its 200x200 square wherever expansion anchors it. */
  .pet-anchor { position: absolute; width: 200px; height: 200px; }
  .panel-slot {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 384px;
    display: flex;
  }
</style>
