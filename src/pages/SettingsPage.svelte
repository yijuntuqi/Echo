<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { settingsStore } from '$lib/stores/settings';
  import { systemStore } from '$lib/stores/system';
  import { getSettings, updateSettings, checkUpdates, installUpdate } from '$lib/api/commands';
  import { on } from '$lib/api/events';
  import Button from '$lib/components/ui/Button.svelte';
  import TextField from '$lib/components/ui/TextField.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';

  const THEMES = [
    ['light', '☀️ 浅色'],
    ['dark', '🌙 深色'],
    ['auto', '💻 跟随系统'],
  ] as const;

  let saving = $state(false);
  let notice = $state('');
  let loadError = $state('');
  // Auto-update: progress bar while the pending release downloads.
  let installing = $state(false);
  let installPercent = $state<number | null>(null);
  let unlistenProgress: (() => void) | undefined;

  onMount(async () => {
    try {
      settingsStore.load(await getSettings());
      const u = await checkUpdates();
      systemStore.setAppVersion(u.version);
      systemStore.setUpdateAvailable(u.available, u.version);
    } catch (e) {
      // Visible on the page: a silently failing settings page looks like a
      // blank screen to the user.
      loadError = String(e);
      console.warn('settings unavailable:', e);
    }
    try {
      unlistenProgress = await on('update:progress', (p) => {
        installPercent = p.total
          ? Math.min(100, Math.round((p.downloaded / p.total) * 100))
          : null;
      });
    } catch {
      /* non-Tauri context: no progress events */
    }
  });

  onDestroy(() => unlistenProgress?.());

  async function installUpdateNow() {
    installing = true;
    try {
      // On success the backend relaunches the app; on Windows the NSIS
      // installer ends this process, so reaching the catch below means it
      // did not go through.
      await installUpdate();
    } catch (e) {
      notice = '更新失败：' + e;
    } finally {
      installing = false;
      installPercent = null;
    }
  }

  function setTheme(value: (typeof THEMES)[number][0]) {
    settingsStore.update({ theme: value });
    document.documentElement.dataset.theme = value;
  }

  async function save() {
    saving = true;
    notice = '';
    try {
      await updateSettings(settingsStore.settings);
      notice = '已保存';
    } catch (e) {
      notice = '保存失败：' + e;
    } finally {
      saving = false;
      setTimeout(() => (notice = ''), 2500);
    }
  }
</script>

<div class="settings-page">
  <header class="top-bar">
    <h1>⚙️ 设置</h1>
    <a href="#/">← 返回</a>
  </header>

  {#if loadError}
    <p class="load-error" role="alert">设置加载失败：{loadError}</p>
  {/if}

  <section class="card">
    <h2>👤 个人资料</h2>
    <TextField label="昵称" bind:value={settingsStore.settings.nickname} placeholder="Echo 该怎么称呼你？" />
    <TextField label="生日" type="date" bind:value={settingsStore.settings.birthday} />
  </section>

  <section class="card">
    <h2>🎨 外观</h2>
    <div class="theme-row">
      {#each THEMES as [value, label]}
        <button
          class="theme-btn"
          class:active={settingsStore.settings.theme === value}
          onclick={() => setTheme(value)}
        >{label}</button>
      {/each}
    </div>
  </section>

  <section class="card">
    <h2>🔔 通知与启动</h2>
    <Toggle label="生日 / 周年 / 每日回顾通知" bind:checked={settingsStore.settings.notifications} />
    <Toggle label="开机自动启动" bind:checked={settingsStore.settings.auto_start} />
  </section>

  <section class="card">
    <h2>🤖 AI 设置</h2>
    <TextField
      label="个人 API Key（可选）"
      type="password"
      bind:value={settingsStore.settings.user_api_key}
      placeholder="sk-..."
    />
    <TextField
      label="接口地址（可选）"
      bind:value={settingsStore.settings.user_base_url}
      placeholder="https://api.openai.com/v1"
    />
    <p class="hint">
      填入 Key 后使用你自己的额度，不受每日 200 次限制；留空则使用内置共享额度。
      接口地址需为 OpenAI 兼容格式，仅搭配自己的 Key 生效，留空使用内置服务商。
    </p>
  </section>

  <section class="card">
    <h2>ℹ️ 关于</h2>
    <p>版本 {systemStore.appVersion}</p>
    {#if systemStore.updateAvailable}
      <p>🆕 新版本{systemStore.updateVersion ? ` v${systemStore.updateVersion}` : ''}可用</p>
      {#if installPercent !== null}
        <div
          class="update-bar"
          role="progressbar"
          aria-valuenow={installPercent}
          aria-valuemin={0}
          aria-valuemax={100}
        >
          <div class="update-fill" style="width: {installPercent}%"></div>
        </div>
        <p class="update-note">下载中… {installPercent}%（完成后自动重启）</p>
      {:else}
        <Button onclick={installUpdateNow} disabled={installing}>
          {installing ? '正在更新…' : '下载并安装'}
        </Button>
      {/if}
    {:else}
      <p>✅ 已是最新版本</p>
    {/if}
  </section>

  <div class="footer">
    <Button onclick={save} disabled={saving}>{saving ? '保存中…' : '保存设置'}</Button>
    {#if notice}<span class="notice">{notice}</span>{/if}
  </div>
</div>

<style>
  .settings-page {
    padding: 20px;
    max-width: 600px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .top-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--color-border);
  }
  .top-bar h1 { font-size: 1.4rem; font-weight: 700; }
  .top-bar a { color: var(--color-text-muted); text-decoration: none; font-size: 14px; }
  .top-bar a:hover { color: var(--color-accent); }

  .card {
    background: var(--color-bg-panel);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .card h2 { font-size: 1rem; color: var(--color-text); }
  .card p { font-size: 13px; color: var(--color-text-muted); margin: 0; }

  .theme-row { display: flex; gap: 8px; }
  .theme-btn {
    flex: 1;
    padding: 10px;
    font: inherit;
    font-size: 13px;
    color: var(--color-text);
    background: var(--color-bg-input);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .theme-btn.active {
    border-color: var(--color-accent);
    color: var(--color-accent);
    background: var(--color-accent-alpha);
  }

  .hint { font-size: 12px !important; }
  .update-bar {
    height: 6px;
    border-radius: 3px;
    background: var(--color-border);
    overflow: hidden;
  }
  .update-fill {
    height: 100%;
    background: var(--color-accent);
    transition: width 0.2s ease;
  }
  .update-note { font-size: 12px !important; }
  .load-error {
    margin: 0;
    padding: 8px 12px;
    background: rgba(231, 76, 60, 0.08);
    color: #e74c3c;
    font-size: 13px;
    border-radius: var(--radius-sm);
    overflow-wrap: anywhere;
  }
  .footer { display: flex; align-items: center; gap: 12px; }
  .notice { font-size: 13px; color: var(--color-accent); }
</style>
