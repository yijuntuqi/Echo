<script lang="ts">
    import { onMount } from 'svelte';
    import { page } from '$app/stores';
    import PetOverlay from '$lib/components/pet/PetOverlay.svelte';
    import ChatPanel from '$lib/components/chat/ChatPanel.svelte';
    import { showPetOverlay, getSettings } from '$lib/api/commands';
    import { petStore } from '$lib/stores/pet';
    import { chatStore } from '$lib/stores/chat';
    
    onMount(async () => {
        try {
            const settings = await getSettings();
            document.documentElement.dataset.theme = settings.theme || 'auto';
            await showPetOverlay();
        } catch (e) {
            console.warn('Failed to init:', e);
        }
    });
</script>

<div class="app-root" data-theme="auto">
    <PetOverlay />
    <ChatPanel />
    <main class="main-content">
        <slot />
    </main>
</div>

<style>
    .app-root {
        min-height: 100vh;
        background: var(--color-bg-app);
        color: var(--color-text);
        transition: background 0.3s, color 0.3s;
        font-family: var(--font-sans);
    }
    
    .main-content {
        padding: 20px;
        max-width: 800px;
        margin: 0 auto;
    }
    
    :global([data-theme="light"]) {
        --color-bg-app: #fafafa;
        --color-bg-panel: #ffffff;
        --color-bg-input: #ffffff;
        --color-border: #e5e5e5;
        --color-text: #1a1a1a;
        --color-text-muted: #737373;
        --color-accent: #ff6b35;
        --color-accent-hover: #e85d2a;
        --color-accent-alpha: rgba(255, 107, 53, 0.15);
        --font-sans: 'Inter', 'Noto Sans SC', system-ui, sans-serif;
        --font-mono: 'JetBrains Mono', 'Fira Code', monospace;
        --radius-lg: 16px;
        --radius-md: 12px;
        --radius-sm: 8px;
        --shadow-soft: 0 4px 20px rgba(0,0,0,0.08);
        --shadow-strong: 0 20px 40px rgba(0,0,0,0.15);
    }
    
    :global([data-theme="dark"]) {
        --color-bg-app: #121212;
        --color-bg-panel: #1e1e1e;
        --color-bg-input: #2a2a2a;
        --color-border: #333;
        --color-text: #f5f5f5;
        --color-text-muted: #a3a3a3;
        --color-accent: #ff9f7a;
        --color-accent-hover: #ffb396;
        --color-accent-alpha: rgba(255, 159, 122, 0.15);
        --font-sans: 'Inter', 'Noto Sans SC', system-ui, sans-serif;
        --font-mono: 'JetBrains Mono', 'Fira Code', monospace;
        --radius-lg: 16px;
        --radius-md: 12px;
        --radius-sm: 8px;
        --shadow-soft: 0 4px 20px rgba(0,0,0,0.3);
        --shadow-strong: 0 20px 40px rgba(0,0,0,0.4);
    }
    
    * { box-sizing: border-box; margin: 0; padding: 0; }
    html, body, #app { height: 100%; }
    ::selection { background: var(--color-accent-alpha); color: var(--color-text); }
    ::-webkit-scrollbar { width: 8px; height: 8px; }
    ::-webkit-scrollbar-track { background: transparent; }
    ::-webkit-scrollbar-thumb { background: var(--color-border); border-radius: 4px; }
    ::-webkit-scrollbar-thumb:hover { background: var(--color-text-muted); }
</style>