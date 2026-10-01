<script lang="ts">
  import { completeOnboarding } from '$lib/api/commands';
  import { goto } from '$lib/router';
  import Button from '$lib/components/ui/Button.svelte';
  import TextField from '$lib/components/ui/TextField.svelte';
  import Toggle from '$lib/components/ui/Toggle.svelte';

  const TOTAL = 3;
  const THEMES = [
    ['light', '☀️ 浅色'],
    ['dark', '🌙 深色'],
    ['auto', '💻 跟随系统'],
  ] as const;

  let step = $state(1);
  let nickname = $state('');
  let birthday = $state('');
  let userApiKey = $state('');
  let userBaseUrl = $state('');
  let theme = $state<'light' | 'dark' | 'auto'>('auto');
  let notifications = $state(true);
  let autoStart = $state(true);
  let error = $state('');
  let busy = $state(false);

  function next() {
    error = '';
    if (step === 1) {
      if (!nickname.trim()) { error = '请填写昵称'; return; }
      if (!birthday) { error = '请选择生日（生日祝福需要）'; return; }
    }
    if (step < TOTAL) { step++; return; }
    finish();
  }

  async function finish() {
    busy = true;
    try {
      await completeOnboarding(
        { nickname: nickname.trim(), birthday, install_date: new Date().toISOString().slice(0, 10) },
        {
          nickname: nickname.trim(),
          birthday,
          theme,
          notifications,
          auto_start: autoStart,
          user_api_key: userApiKey || undefined,
          user_base_url: userBaseUrl.trim() || undefined,
          model_preference: 'auto',
        },
      );
      document.documentElement.dataset.theme = theme;
      goto('/');
    } catch (e) {
      error = '保存失败：' + e;
    } finally {
      busy = false;
    }
  }
</script>

<div class="onboarding">
  <div class="progress" aria-label={`第 ${step} 步，共 ${TOTAL} 步`}>
    {#each Array(TOTAL) as _, i}
      <span class="dot" class:on={i + 1 <= step}></span>
    {/each}
  </div>

  {#if step === 1}
    <h2>你好，我是 Echo 🥚</h2>
    <p class="lead">先告诉我怎么称呼你，和你的生日 —— 我会记在心里。</p>
    <TextField bind:value={nickname} placeholder="你的昵称" />
    <TextField type="date" bind:value={birthday} />
  {:else if step === 2}
    <h2>外观与提醒</h2>
    <div class="theme-row">
      {#each THEMES as [value, label]}
        <button
          class="theme-btn"
          class:active={theme === value}
          onclick={() => (theme = value)}
        >{label}</button>
      {/each}
    </div>
    <Toggle label="生日 / 周年 / 每日回顾通知" bind:checked={notifications} />
    <Toggle label="开机自动启动" bind:checked={autoStart} />
  {:else}
    <h2>AI 设置（可选）</h2>
    <p class="lead">留空则使用内置共享额度（每日 200 次）。填入自己的 Key 可解除限制。</p>
    <TextField type="password" bind:value={userApiKey} placeholder="sk-..." />
    <TextField bind:value={userBaseUrl} placeholder="接口地址（可选，默认内置）" />
    <p class="tip">接口地址需为 OpenAI 兼容格式（如 https://api.openai.com/v1），仅搭配自己的 Key 生效。</p>
  {/if}

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="actions">
    <Button variant="secondary" onclick={() => (step = Math.max(1, step - 1))} disabled={step === 1 || busy}>
      上一步
    </Button>
    <Button onclick={next} disabled={busy}>{step === TOTAL ? '完成' : '下一步'}</Button>
  </div>
</div>

<style>
  .onboarding {
    max-width: 460px;
    margin: 64px auto;
    padding: 32px;
    background: var(--color-bg-panel);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .progress { display: flex; justify-content: center; gap: 8px; margin-bottom: 4px; }
  .dot {
    width: 10px; height: 10px; border-radius: 50%;
    background: var(--color-border); transition: all 0.3s;
  }
  .dot.on { background: var(--color-accent); transform: scale(1.15); }

  h2 { font-size: 1.35rem; text-align: center; }
  .lead { font-size: 13px; color: var(--color-text-muted); text-align: center; margin: 0; }

  .theme-row { display: flex; gap: 8px; }
  .theme-btn {
    flex: 1; padding: 10px; font: inherit; font-size: 13px;
    color: var(--color-text);
    background: var(--color-bg-input);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm); cursor: pointer;
  }
  .theme-btn.active {
    border-color: var(--color-accent);
    color: var(--color-accent);
    background: var(--color-accent-alpha);
  }

  .error { font-size: 13px; color: #e74c3c; margin: 0; }
  .tip { font-size: 12px; color: var(--color-text-muted); margin: 0; }
  .actions { display: flex; justify-content: space-between; margin-top: 8px; }
</style>
