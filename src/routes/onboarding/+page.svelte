<!-- Onboarding Wizard -->
<script lang="ts">
    import { goto } from '$app/navigation';
    import { completeOnboarding } from '$lib/api/commands';
    import { Button } from '@bits-ui/components/button';
    import { Input } from '@bits-ui/components/input';
    import { Switch } from '@bits-ui/components/switch';
    import { Select } from '@bits-ui/components/select';
    
    let step = $state(1);
    const maxSteps = 4;
    
    let nickname = $state('');
    let birthday = $state('');
    let theme: 'light' | 'dark' | 'auto' = $state('auto');
    let notifications = $state(true);
    let autoStart = $state(true);
    let userApiKey = $state('');
    let modelPreference: 'daily' | 'premium' | 'auto' = $state('auto');
    
    async function nextStep() {
        if (step === 1 && (!nickname || !birthday)) return alert('请填写昵称和生日');
        if (step === maxSteps) {
            await completeOnboarding(
                { nickname, birthday, install_date: new Date().toISOString().split('T')[0], settings_json: '{}' },
                { theme, notifications, auto_start, user_api_key: userApiKey || undefined, model_preference: modelPreference, backup_password_hash: undefined }
            );
            goto('/');
            return;
        }
        step++;
    }
    
    function prevStep() { if (step > 1) step--; }
</script>

<div class="onboarding" animate:slide={{ x: 50, duration: 300 }}>
    <div class="progress">
        {#each Array(maxSteps) as _, i}
            <div class="step-dot" class:active={i + 1 <= step} />
        {/each}
    </div>
    
    {#if step === 1}
        <h2>你好，我是 Echo 🥚</h2>
        <p>先告诉我怎么称呼你，和你的生日（我会记得）</p>
        <Input.Root bind:value={nickname} placeholder="你的昵称" />
        <Input.Root type="date" bind:value={birthday} />
    
    {:else if step === 2}
        <h2>外观偏好</h2>
        <Select.Root bind:value={theme}>
            <Select.Trigger><Select.Value /></Select.Trigger>
            <Select.Content>
                <Select.Item value="light">☀️ 浅色</Select.Item>
                <Select.Item value="dark">🌙 深色</Select.Item>
                <Select.Item value="auto">💻 跟随系统</Select.Item>
            </Select.Content>
        </Select.Root>
    
    {:else if step === 3}
        <h2>通知与自启</h2>
        <label class="toggle-row"><Switch.Root bind:checked={notifications} /> 生日/周年/每日回顾通知</label>
        <label class="toggle-row"><Switch.Root bind:checked={autoStart} /> 开机自动启动</label>
    
    {:else if step === 4}
        <h2>AI 设置（可选）</h2>
        <p class="hint">填入你自己的 API Key 可解除每日 200 次限制，留空使用内置共享额度</p>
        <Input.Root type="password" bind:value={userApiKey} placeholder="sk-... (ChatAnywhere / OpenAI 兼容)" />
        <Select.Root bind:value={modelPreference}>
            <Select.Trigger><Select.Value /></Select.Trigger>
            <Select.Content>
                <Select.Item value="daily">💬 日常闲聊</Select.Item>
                <Select.Item value="premium">🧠 重要节点深度思考</Select.Item>
                <Select.Item value="auto">🤖 智能切换</Select.Item>
            </Select.Content>
        </Select.Root>
    {/if}
    
    <div class="actions">
        <Button.Root variant="secondary" on:click={prevStep} disabled={step === 1}>上一步</Button.Root>
        <Button.Root on:click={nextStep}>{step === maxSteps ? '完成' : '下一步'}</Button.Root>
    </div>
</div>

<style>
    .onboarding { max-width: 480px; margin: 60px auto; padding: 32px; background: var(--color-bg-panel); border-radius: var(--radius-lg); border: 1px solid var(--color-border); text-align: center; }
    .progress { display: flex; justify-content: center; gap: 8px; margin-bottom: 32px; }
    .step-dot { width: 10px; height: 10px; border-radius: 50%; background: var(--color-border); transition: all 0.3s; }
    .step-dot.active { background: var(--color-accent); transform: scale(1.2); }
    .hint { font-size: 13px; color: var(--color-text-muted); margin-bottom: 16px; text-align: left; }
    .toggle-row { display: flex; justify-content: space-between; align-items: center; padding: 12px 0; border-bottom: 1px solid var(--color-border); text-align: left; }
    .actions { display: flex; justify-content: space-between; margin-top: 32px; }
</style>