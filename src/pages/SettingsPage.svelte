<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { settingsStore } from '$lib/stores/settings';
  import { systemStore } from '$lib/stores/system';
  import { getSettings, updateSettings, checkUpdates, installUpdate, exportBackup, resetDatabase } from '$lib/api/commands';
  // Renamed: the page's own `save()` below is the settings saver.
  import { save as saveDialog } from '@tauri-apps/plugin-dialog';
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

  // --- data export ----------------------------------------------------------
  let exporting = $state(false);

  async function exportData() {
    exporting = true;
    notice = '';
    try {
      const stamp = new Date().toISOString().slice(0, 10).replace(/-/g, '');
      // A cancel (null) is a normal outcome, not an error.
      const path = await saveDialog({
        defaultPath: `echo-backup-${stamp}.echo.db`,
        filters: [{ name: 'Echo 备份', extensions: ['echo.db', 'db'] }],
      });
      if (!path) return;
      const saved = await exportBackup(path);
      notice = `已导出到：${saved}`;
    } catch (e) {
      notice = '导出失败：' + e;
    } finally {
      exporting = false;
    }
  }

  // --- danger zone: two-step reset ------------------------------------------
  let resetArmed = $state(false);
  let resetting = $state(false);
  let resetTimer: ReturnType<typeof setTimeout> | undefined;

  function onResetClick() {
    if (resetting) return;
    if (!resetArmed) {
      resetArmed = true;
      resetTimer = setTimeout(() => (resetArmed = false), 3000);
      return;
    }
    clearTimeout(resetTimer);
    resetArmed = false;
    void doReset();
  }

  async function doReset() {
    resetting = true;
    try {
      // Success never returns here: the backend deletes the database and
      // restarts the app. Reaching this line means it did not go through.
      await resetDatabase();
      notice = '重置未完成，请手动重启应用后重试';
    } catch (e) {
      notice = '重置失败：' + e;
    } finally {
      resetting = false;
    }
  }

  // Light format checks — advisory only, never block saving.
  const keyWarning = $derived.by(() => {
    const key = (settingsStore.settings.user_api_key ?? '').trim();
    return key && !key.startsWith('sk-')
      ? '常见 Key 以 “sk-” 开头，当前格式可能不对（仍可保存）'
      : '';
  });
  const urlWarning = $derived.by(() => {
    const url = (settingsStore.settings.user_base_url ?? '').trim();
    return url && !/^https?:\/\//i.test(url)
      ? '接口地址需以 http:// 或 https:// 开头（仍可保存）'
      : '';
  });

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
    {#if keyWarning}<p class="warn" role="alert">⚠️ {keyWarning}</p>{/if}
    {#if urlWarning}<p class="warn" role="alert">⚠️ {urlWarning}</p>{/if}
    <label class="field">
      <span class="field-label">回复模型</span>
      <select bind:value={settingsStore.settings.model_preference}>
        <option value="auto">自动（推荐 · 日常轻量模型）</option>
        <option value="premium">高级模型（更聪明 · 可能更慢）</option>
      </select>
    </label>
    <p class="hint">
      填入 Key 后使用你自己的额度，不受每日 200 次限制；留空则使用内置共享额度。
      接口地址需为 OpenAI 兼容格式，仅搭配自己的 Key 生效，留空使用内置服务商。
    </p>
  </section>

  <section class="card">
    <h2>💾 数据</h2>
    <p>备份是一个仍被加密的数据库快照，可在更换电脑后用恢复功能还原。</p>
    <Button onclick={exportData} disabled={exporting}>
      {exporting ? '导出中…' : '导出备份数据'}
    </Button>
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

  <section class="card danger">
    <h2>⚠️ 危险区</h2>
    <p>重置将永久删除全部本地数据（对话、记忆、事件），无法恢复。应用会自动重启并进入重新设置。</p>
    <button class="danger-btn" onclick={onResetClick} disabled={resetting}>
      {resetting ? '正在重置…' : resetArmed ? '再点一次确认重置' : '重置数据库'}
    </button>
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
  .card p.warn { font-size: 12px; color: #e67e22; }
  .field { display: flex; flex-direction: column; gap: 6px; }
  .field-label { font-size: 13px; color: var(--color-text); }
  .field select {
    padding: 9px 12px; font: inherit; font-size: 14px;
    color: var(--color-text);
    background: var(--color-bg-input);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
  }
  .field select:focus { outline: none; border-color: var(--color-accent); }
  .card.danger { border-color: rgba(231, 76, 60, 0.45); }
  .danger-btn {
    align-self: flex-start;
    padding: 9px 16px; font-size: 13px; font-weight: 500;
    color: #e74c3c; background: none;
    border: 1px solid #e74c3c;
    border-radius: var(--radius-sm); cursor: pointer;
  }
  .danger-btn:hover:not(:disabled) { background: #e74c3c; color: #fff; }
  .danger-btn:disabled { opacity: 0.5; cursor: not-allowed; }
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
