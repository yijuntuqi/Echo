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
  // Any uncaught error in either window shows here: a silent white screen is
  // undiagnosable from the outside, a visible message is half the fix.
  let fatalMessage = $state('');

  onMount(() => {
    // Keep the teardown synchronous; do the async work inside.
    let disposed = false;
    const cleanups: (() => void)[] = [];

    const showFatal = (msg: string) => {
      fatalMessage = msg.slice(0, 300);
      window.setTimeout(() => (fatalMessage = ''), 12000);
    };
    const onError = (e: ErrorEvent) => showFatal(e.message || String(e.error));
    const onReject = (e: PromiseRejectionEvent) => showFatal(String(e.reason));
    window.addEventListener('error', onError);
    window.addEventListener('unhandledrejection', onReject);

    // Never let a hung backend command pin the shell on the boot screen:
    // after 8s show the UI anyway (commands keep resolving in background).
    const bootTimeout = window.setTimeout(() => {
      if (!disposed) booting = false;
    }, 8000);

    void (async () => {
      // Register event listeners BEFORE anything else: tray events can fire
      // while the onboarding/settings commands are still resolving, and a
      // listener registered after that misses them for good.
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

      // Tray 「主窗口」/「设置」 both surface this window; the payload picks
      // the page they land on. Unknown routes are ignored.
      try {
        const unlistenNav = await on('nav:goto', (raw) => {
          if (raw === '/' || raw === '/dashboard' || raw === '/settings') goto(raw);
        });
        if (disposed) unlistenNav();
        else cleanups.push(unlistenNav);
      } catch (e) {
        console.warn('nav:goto listener unavailable:', e);
      }

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
        window.clearTimeout(bootTimeout);
        if (!disposed) booting = false;
      }
    })();

    return () => {
      disposed = true;
      window.clearTimeout(bootTimeout);
      cleanups.forEach((fn) => fn());
      window.removeEventListener('error', onError);
      window.removeEventListener('unhandledrejection', onReject);
    };
  });
</script>

<div class="app-root">
  {#if fatalMessage}
    <div class="fatal" role="alert">⚠ 页面出错：{fatalMessage}</div>
  {/if}
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
  .fatal {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 9999;
    padding: 8px 16px;
    background: rgba(231, 76, 60, 0.92);
    color: #fff;
    font-size: 12.5px;
    overflow-wrap: anywhere;
  }
</style>
