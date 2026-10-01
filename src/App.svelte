<!-- App shell: hosts the hash router and global overlays. The chat panel
     lives in the pet window so it can follow the pet around. -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { router, goto } from '$lib/router';
  import { getSettings, getOnboardingStatus } from '$lib/api/commands';
  import { on } from '$lib/api/events';
  import { settingsStore } from '$lib/stores/settings';
  import { moodStore } from '$lib/stores/mood';
  import { petStore } from '$lib/stores/pet';
  import { evolutionStore } from '$lib/stores/evolution';

  import HomePage from './pages/HomePage.svelte';
  import DashboardPage from './pages/DashboardPage.svelte';
  import SettingsPage from './pages/SettingsPage.svelte';
  import OnboardingPage from './pages/OnboardingPage.svelte';

  let booting = $state(true);

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

      // A finished turn carries today's emotional read; keep the heatmap live.
      try {
        const unlistenMood = await on('mood:updated', (m) => {
          moodStore.upsert({ date: m.date, emotion: m.emotion, weight: m.weight, source: 'auto' });
        });
        if (disposed) unlistenMood();
        else cleanups.push(unlistenMood);
      } catch (e) {
        console.warn('mood:updated listener unavailable:', e);
      }

      // The pet grew: mirror the transition into the dashboard store and
      // replay the evolve animation.
      try {
        const unlistenEvolution = await on('evolution:triggered', (e) => {
          evolutionStore.applyEvolution({
            timestamp: new Date().toISOString(),
            from_stage: e.from_stage,
            to_stage: e.to_stage,
            trigger_type: e.trigger,
            personality_vector: e.personality,
            score: e.score,
          });
          petStore.setStage(e.to_stage);
        });
        if (disposed) unlistenEvolution();
        else cleanups.push(unlistenEvolution);
      } catch (e) {
        console.warn('evolution:triggered listener unavailable:', e);
      }
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
