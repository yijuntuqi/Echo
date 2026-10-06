<script lang="ts">
  import { completeOnboarding, resetDatabase } from '$lib/api/commands';
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
  // The old-database escape hatch: the file exists but the keychain entry
  // can no longer open it, which used to dead-end on "wrong password".
  let showReset = $state(false);
  let resetting = $state(false);

  // Light format checks — advisory only; saving stays possible.
  const keyWarning = $derived.by(() => {
    const key = userApiKey.trim();
    return key && !key.startsWith('sk-')
      ? '常见 Key 以 “sk-” 开头，当前格式可能不对（仍可继续）'
      : '';
  });
  const urlWarning = $derived.by(() => {
    const url = userBaseUrl.trim();
    return url && !/^https?:\/\//i.test(url)
      ? '接口地址需以 http:// 或 https:// 开头（仍可继续）'
      : '';
  });

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
          // Empty string = "not set": the Rust side falls back to the shared
          // key for any empty key, so no need to send undefined here.
          user_api_key: userApiKey.trim(),
          user_base_url: userBaseUrl.trim(),
          model_preference: 'auto',
        },
      );
      document.documentElement.dataset.theme = theme;
      goto('/');
    } catch (e) {
      const msg = typeof e === 'string' ? e : String(e);
      if (msg.includes('wrong password') || msg.includes('not a database')) {
        // An encrypted database from a previous install exists and the freshly
        // generated password cannot open it. Offer the reset instead of a
        // dead-end error the user can only hit again.
        showReset = true;
      } else {
        error = '保存失败：' + msg;
      }
    } finally {
      busy = false;
    }
  }

  async function doReset() {
    resetting = true;
    try {
      // Success never resolves: the backend deletes the database and
      // restarts the app into a clean onboarding. Reaching this line means
      // the restart did not happen.
      await resetDatabase();
      error = '重置未完成，请手动重启应用后重试';
    } catch (e) {
      error = '重置失败：' + e;
    } finally {
      resetting = false;
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
    {#if keyWarning}<p class="warn" role="alert">⚠️ {keyWarning}</p>{/if}
    {#if urlWarning}<p class="warn" role="alert">⚠️ {urlWarning}</p>{/if}
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

{#if showReset}
  <div class="reset-backdrop" role="alertdialog" aria-modal="true" aria-label="重置数据库确认">
    <div class="reset-dialog">
      <h3>⚠️ 旧数据库无法打开</h3>
      <p>
        本机已有一份 Echo 数据库，但当前无法解密（密码不匹配或文件损坏）。
        重置会删除它的全部内容（对话、记忆、事件），此操作无法恢复。
      </p>
      <div class="reset-actions">
        <Button variant="secondary" onclick={() => (showReset = false)} disabled={resetting}>
          取消
        </Button>
        <button class="danger-btn" onclick={doReset} disabled={resetting}>
          {resetting ? '正在重置…' : '重置并重新开始'}
        </button>
      </div>
    </div>
  </div>
{/if}

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
  .warn { font-size: 12px; color: #e67e22; margin: 0; }

  .reset-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.45);
  }
  .reset-dialog {
    width: min(420px, calc(100vw - 48px));
    padding: 24px;
    background: var(--color-bg-panel);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-strong);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .reset-dialog h3 { margin: 0; font-size: 1.1rem; }
  .reset-dialog p { margin: 0; font-size: 13px; color: var(--color-text-muted); line-height: 1.6; }
  .reset-actions { display: flex; justify-content: flex-end; gap: 10px; }
  .danger-btn {
    padding: 9px 16px; font-size: 13px; font-weight: 500;
    color: #e74c3c; background: none;
    border: 1px solid #e74c3c;
    border-radius: var(--radius-sm); cursor: pointer;
  }
  .danger-btn:hover:not(:disabled) { background: #e74c3c; color: #fff; }
  .danger-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .tip { font-size: 12px; color: var(--color-text-muted); margin: 0; }
  .actions { display: flex; justify-content: space-between; margin-top: 8px; }
</style>
