<!-- PetAvatar: SVG state machine + drag + click for the pet window -->
<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { getCurrentWindow, PhysicalPosition } from '@tauri-apps/api/window';
  import { openChat } from '$lib/api/commands';
  import { petStore } from '$lib/stores/pet';

  const dispatch = createEventDispatcher<{
    openChat: void;
    contextMenu: { x: number; y: number };
  }>();

  // Per-stage inline SVG art. Replaced by real asset files once they exist.
  const STAGE_ART: Record<string, { body: string; accent: string }> = {
    egg:     { body: '#fff8dc', accent: '#e8d9a0' },
    child:   { body: '#ffe4b5', accent: '#e6b980' },
    teen:    { body: '#ffd700', accent: '#d4a900' },
    adult:   { body: '#ff8c00', accent: '#d16f00' },
    ultimate:{ body: '#ff4500', accent: '#c23600' },
  };

  let svgMarkup = $derived.by(() => {
    const art = STAGE_ART[petStore.stage] ?? STAGE_ART.egg;
    const eyes = petStore.animation === 'sleep'
      ? `<path d="M78 96 q6 6 12 0 M110 96 q6 6 12 0" stroke="#333" stroke-width="3" fill="none" stroke-linecap="round"/>`
      : `<circle cx="84" cy="96" r="5" fill="#333"/><circle cx="116" cy="96" r="5" fill="#333"/>`;
    return `<svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
      <ellipse cx="100" cy="168" rx="46" ry="8" fill="rgba(0,0,0,0.10)"/>
      <path d="M100 24 C134 24 156 56 156 96 C156 132 132 154 100 154 C68 154 44 132 44 96 C44 56 66 24 100 24 Z"
            fill="${art.body}" stroke="${art.accent}" stroke-width="3"/>
      ${eyes}
      <path d="M88 118 q12 10 24 0" stroke="#333" stroke-width="3" fill="none" stroke-linecap="round"/>
    </svg>`;
  });

  // Animation is CSS-driven off the store's animation state.
  let animClass = $derived(`anim-${petStore.animation}`);

  let dragging = $state(false);
  let dragOrigin = { x: 0, y: 0 };
  // Where this press began; movement beyond DRAG_THRESHOLD px reclassifies
  // the gesture from click to drag and suppresses the trailing click event.
  let pressOrigin = { x: 0, y: 0 };
  const DRAG_THRESHOLD = 4;
  let moved = $state(false);

  function startDrag(e: MouseEvent) {
    if (e.button !== 0 || petStore.clickThrough) return;
    dragging = true;
    moved = false;
    dragOrigin = { x: e.screenX, y: e.screenY };
    pressOrigin = { x: e.screenX, y: e.screenY };
    petStore.playAnimation('walk');
  }

  async function onDrag(e: MouseEvent) {
    if (!dragging) return;
    if (
      !moved &&
      Math.hypot(e.screenX - pressOrigin.x, e.screenY - pressOrigin.y) < DRAG_THRESHOLD
    ) {
      return;
    }
    moved = true;
    const dx = e.screenX - dragOrigin.x;
    const dy = e.screenY - dragOrigin.y;
    if (dx === 0 && dy === 0) return;
    dragOrigin = { x: e.screenX, y: e.screenY };
    try {
      const win = getCurrentWindow();
      const pos = await win.outerPosition();
      await win.setPosition(new PhysicalPosition(pos.x + dx, pos.y + dy));
    } catch {
      // Not running inside Tauri (e.g. `vite dev` in a plain browser tab).
    }
  }

  function endDrag() {
    if (!dragging) return;
    dragging = false;
    petStore.playAnimation('idle');
  }

  async function onClick() {
    // A real drag ends with a click event on the pet; drop it instead of
    // opening chat every time the pet is repositioned.
    if (moved) {
      moved = false;
      return;
    }
    petStore.playAnimation('react');
    petStore.touch();
    // The chat panel lives in the main window; ask the backend to surface it.
    openChat().catch(() => dispatch('openChat'));
  }

  function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    dispatch('contextMenu', { x: e.screenX, y: e.screenY });
  }
</script>

<svelte:window
  onmousedown={startDrag}
  onmousemove={onDrag}
  onmouseup={endDrag}
  onmouseleave={endDrag}
/>

<div
  class="pet-avatar {animClass}"
  class:dragging
  onclick={onClick}
  oncontextmenu={onContextMenu}
  onkeydown={(e) => e.key === 'Enter' && onClick()}
  role="button"
  tabindex="0"
  aria-label="Echo 宠物"
>
  {@html svgMarkup}

  {#if petStore.animation === 'evolve'}
    <span class="evolve-ring" aria-hidden="true"></span>
  {/if}
</div>

<style>
  .pet-avatar {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
    cursor: grab;
    user-select: none;
    -webkit-user-select: none;
    -webkit-app-region: no-drag;
  }
  .pet-avatar.dragging { cursor: grabbing; }
  .pet-avatar :global(svg) { width: 100%; height: 100%; pointer-events: none; }

  /* --- animation states --- */
  .anim-idle   { animation: breathe 3.2s ease-in-out infinite; }
  .anim-walk   { animation: waddle 0.6s ease-in-out infinite; }
  .anim-sleep  { animation: breathe 4.5s ease-in-out infinite; filter: saturate(0.7); }
  .anim-talk   { animation: chatter 0.28s ease-in-out infinite; }
  .anim-react  { animation: pop 0.45s ease-out; }
  .anim-evolve { animation: evolve 1.5s ease-out; }

  @keyframes breathe { 0%,100% { transform: scale(1) } 50% { transform: scale(1.03) } }
  @keyframes waddle  { 0%,100% { transform: rotate(-4deg) } 50% { transform: rotate(4deg) } }
  @keyframes chatter { 0%,100% { transform: translateY(0) scale(1) } 50% { transform: translateY(-2px) scale(1.04) } }
  @keyframes pop     { 0% { transform: scale(1) } 40% { transform: scale(1.18) } 100% { transform: scale(1) } }
  @keyframes evolve  {
    0%   { transform: scale(1) rotate(0); opacity: 1 }
    30%  { transform: scale(0) rotate(180deg); opacity: 0 }
    60%  { transform: scale(1.4) rotate(360deg); opacity: 1 }
    100% { transform: scale(1) rotate(360deg); opacity: 1 }
  }

  .evolve-ring {
    position: absolute;
    inset: 8%;
    border: 3px solid #ffd700;
    border-radius: 50%;
    animation: ring 1.5s ease-out forwards;
    pointer-events: none;
  }
  @keyframes ring {
    from { opacity: 1; transform: scale(0.75) }
    to   { opacity: 0; transform: scale(1.4) }
  }
</style>
