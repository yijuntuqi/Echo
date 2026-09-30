<script lang="ts">
  import { onMount } from 'svelte';
  import { settingsStore } from '$lib/stores/settings';
  import { systemStore } from '$lib/stores/system';
  import { getSettings, updateSettings, checkUpdates } from '$lib/api/commands';
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

  onMount(async () => {
    try {
      settingsStore.load(await getSettings());
      const u = await checkUpdates();
      systemStore.setAppVersion(u.version);
      systemStore.setUpdateAvailable(u.available);
    } catch (e) {
      console.warn('settings unavailable:', e);
    }
  });

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
    <p class="hint">填入后使用你自己的额度，不受每日 200 次限制；留空则使用内置共享额度。</p>
  </section>

  <section class="card">
    <h2>ℹ️ 关于</h2>
    <p>版本 {systemStore.appVersion}</p>
    <p>{systemStore.updateAvailable ? '🆕 有新版本可用' : '✅ 已是最新版本'}</p>
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
  .footer { display: flex; align-items: center; gap: 12px; }
  .notice { font-size: 13px; color: var(--color-accent); }
</style>
