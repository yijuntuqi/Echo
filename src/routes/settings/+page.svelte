<!-- Settings Page -->
<script lang="ts">
    import { settingsStore } from '$lib/stores/settings';
    import { systemStore } from '$lib/stores/system';
    import { getSettings, updateSettings, exportBackup, importBackup, checkUpdates } from '$lib/api/commands';
    import { Button } from '@bits-ui/components/button';
    import { Input } from '@bits-ui/components/input';
    import { Switch } from '@bits-ui/components/switch';
    import { Select } from '@bits-ui/components/select';
    import { onMount } from 'svelte';
    
    onMount(async () => {
        try {
            const s = await getSettings();
            settingsStore.load(s);
            const update = await checkUpdates();
            systemStore.setUpdateAvailable(update.available);
            systemStore.setAppVersion(update.version);
        } catch {}
    });
    
    async function saveSettings() {
        try {
            await updateSettings(settingsStore.settings);
            alert('保存成功');
        } catch (e) { alert('保存失败: ' + e); }
    }
    
    async function handleExport() {
        const pwd = prompt('输入备份密码:');
        if (!pwd) return;
        try {
            const path = await exportBackup(pwd, 'echo_backup_' + Date.now() + '.echo.backup');
            alert('导出成功: ' + path);
        } catch (e) { alert('导出失败: ' + e); }
    }
    
    async function handleImport() {
        const pwd = prompt('输入备份密码:');
        if (!pwd) return;
        // TODO: 文件选择器
        alert('请在文件管理器中选择备份文件');
    }
</script>

<div class="settings-page">
    <h1>⚙️ 设置</h1>
    
    <section class="section">
        <h2>👤 个人资料</h2>
        <div class="field">
            <label>昵称</label>
            <Input.Root bind:value={$settingsStore.settings.nickname} />
        </div>
        <div class="field">
            <label>生日</label>
            <Input.Root type="date" bind:value={$settingsStore.settings.birthday} />
        </div>
    </section>
    
    <section class="section">
        <h2>🎨 外观</h2>
        <div class="field">
            <label>主题</label>
            <Select.Root bind:value={$settingsStore.settings.theme}>
                <Select.Trigger><Select.Value /></Select.Trigger>
                <Select.Content>
                    <Select.Item value="light">☀️ 浅色</Select.Item>
                    <Select.Item value="dark">🌙 深色</Select.Item>
                    <Select.Item value="auto">💻 跟随系统</Select.Item>
                </Select.Content>
            </Select.Root>
        </div>
    </section>
    
    <section class="section">
        <h2>🔔 通知与启动</h2>
        <label class="toggle-row"><Switch.Root bind:checked={$settingsStore.settings.notifications} /> 生日/周年/每日回顾通知</label>
        <label class="toggle-row"><Switch.Root bind:checked={$settingsStore.settings.auto_start} /> 开机自动启动</label>
    </section>
    
    <section class="section">
        <h2>🤖 AI 设置</h2>
        <div class="field">
            <label>个人 API Key (可选)</label>
            <Input.Root type="password" bind:value={$settingsStore.settings.user_api_key} placeholder="sk-... (ChatAnywhere / OpenAI 兼容)" />
            <p class="hint">填入后使用你自己的额度，不受每日 200 次限制</p>
        </div>
        <div class="field">
            <label>模型偏好</label>
            <Select.Root bind:value={$settingsStore.settings.model_preference}>
                <Select.Trigger><Select.Value /></Select.Trigger>
                <Select.Content>
                    <Select.Item value="daily">💬 日常闲聊</Select.Item>
                    <Select.Item value="premium">🧠 重要节点深度思考</Select.Item>
                    <Select.Item value="auto">🤖 智能切换</Select.Item>
                </Select.Content>
            </Select.Root>
        </div>
    </section>
    
    <section class="section">
        <h2>💾 数据备份</h2>
        <div class="actions">
            <Button.Root variant="secondary" on:click={handleExport}>导出加密备份</Button.Root>
            <Button.Root variant="secondary" on:click={handleImport}>恢复备份</Button.Root>
        </div>
        <p class="hint">备份包含所有对话、心情、大事记、进化历史。请妥善保管密码，遗忘无法恢复。</p>
    </section>
    
    <section class="section">
        <h2>ℹ️ 关于</h2>
        <p>版本: {$systemStore.appVersion}</p>
        <p>状态: {$systemStore.updateAvailable ? '🆕 有新版本' : '✅ 已是最新'}</p>
        <Button.Root on:click={saveSettings} class="save-btn">保存所有设置</Button.Root>
    </section>
</div>

<style>
    .settings-page { max-width: 600px; margin: 0 auto; padding: 20px; }
    .section { margin-bottom: 32px; padding: 20px; background: var(--color-bg-panel); border-radius: var(--radius-lg); border: 1px solid var(--color-border); }
    .section h2 { margin-bottom: 16px; font-size: 1.1rem; color: var(--color-text); }
    .field { margin-bottom: 16px; }
    .field label { display: block; margin-bottom: 6px; font-size: 13px; color: var(--color-text-muted); }
    .toggle-row { display: flex; justify-content: space-between; align-items: center; padding: 12px 0; border-bottom: 1px solid var(--color-border); font-size: 14px; }
    .toggle-row:last-child { border-bottom: none; }
    .hint { font-size: 12px; color: var(--color-text-muted); margin-top: 4px; }
    .actions { display: flex; gap: 12px; margin-bottom: 12px; }
    .save-btn { width: 100%; margin-top: 16px; }
</style>