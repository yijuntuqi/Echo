<!-- App shell: hosts the hash router and global overlays -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { router, goto } from '$lib/router';
  import ChatPanel from '$lib/components/chat/ChatPanel.svelte';
  import { getSettings, getOnboardingStatus } from '$lib/api/commands';
  import { on } from '$lib/api/events';
  import { settingsStore } from '$lib/stores/settings';

  import HomePage from './pages/HomePage.svelte';
  import DashboardPage from './pages/DashboardPage.svelte';
  import SettingsPage from './pages/SettingsPage.svelte';
  import OnboardingPage from './pages/OnboardingPage.svelte';

  let booting = $state(true);
  let chatOpen = $state(false);

  onMount(() => {
    // Keep the teardown synchronous; do the async work inside.
    let disposed = false;
    const cleanups: (() => void)[] = [];

    void (async () => {
      try {
        const done = await getOnboardingStatus();
        if (disposed) return;
        if (!done) {
          goto('/onboarding');
        } else {
          const s = await getSettings();
          settingsStore.load(s);
          document.documentElement.dataset.theme = s.theme || 'auto';
        }
      } catch (e) {
        // Backend not ready yet: fall back to home so the shell still renders.
        console.warn('onboarding status unavailable:', e);
      } finally {
        if (!disposed) booting = false;
      }

      // The pet window asks the main window to open the chat panel.
      const unlisten = await on('chat:open', () => {
        chatOpen = true;
      });
      if (disposed) unlisten();
      else cleanups.push(unlisten);
    })();

    return () => {
      disposed = true;
      cleanups.forEach((fn) => fn());
    };
  });
</script>

<div class="app-root">
  {#if booting}
    <div class="booting">正在唤醒 Echo…</div>
  {:else if router.current === '/onboarding'}
    <OnboardingPage />
  {:else}
    <ChatPanel bind:open={chatOpen} />
    {#if router.current === '/'}
      <HomePage />
    {:else if router.current === '/dashboard'}
      <DashboardPage />
    {:else if router.current === '/settings'}
      <SettingsPage />
    {/if}
  {/if}
</div>

<style>
  .app-root {
    min-height: 100vh;
    background: var(--color-bg-app);
    color: var(--color-text);
    font-family: var(--font-sans);
  }
  .booting {
    display: grid;
    place-items: center;
    min-height: 60vh;
    color: var(--color-text-muted);
    font-size: 14px;
  }
</style>
