<script lang="ts">
    import { petStore } from '$lib/stores/pet';
</script>

<div class="main-page">
    <header class="top-bar">
        <h1>Echo</h1>
        <nav>
            <a href="/dashboard">📊 记忆</a>
            <a href="/settings">⚙️ 设置</a>
        </nav>
    </header>
    
    <main>
        {#if !$petStore.lastInteraction}
            <div class="welcome-card" animate:fade>
                <h2>欢迎来到 Echo 的世界 🥚</h2>
                <p>点击桌面上的小宠物，开始你们的第一次对话吧。</p>
                <p class="hint">或按 <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>E</kbd> 唤起</p>
            </div>
        {:else}
            <div class="status-card">
                <h3>Echo 正在陪伴你</h3>
                <p>当前阶段: <strong>{$petStore.stage}</strong></p>
                <p>最后互动: {$petStore.lastInteraction ? $petStore.lastInteraction.toLocaleString() : '从未'}</p>
            </div>
        {/if}
    </main>
</div>

<style>
    .main-page { padding: 20px; }
    .top-bar { display: flex; justify-content: space-between; align-items: center; margin-bottom: 32px; padding-bottom: 16px; border-bottom: 1px solid var(--color-border); }
    .top-bar h1 { font-size: 1.5rem; font-weight: 700; background: linear-gradient(135deg, var(--color-accent), var(--color-accent-hover)); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
    .top-bar nav { display: flex; gap: 16px; }
    .top-bar a { color: var(--color-text-muted); text-decoration: none; font-size: 14px; font-weight: 500; transition: color 0.2s; padding: 8px 12px; border-radius: var(--radius-sm); }
    .top-bar a:hover { color: var(--color-accent); background: var(--color-accent-alpha); }
    
    .welcome-card, .status-card {
        text-align: center;
        padding: 48px 32px;
        background: var(--color-bg-panel);
        border-radius: var(--radius-lg);
        border: 1px solid var(--color-border);
        box-shadow: var(--shadow-soft);
    }
    .welcome-card h2 { font-size: 1.75rem; margin-bottom: 12px; color: var(--color-text); }
    .welcome-card p { color: var(--color-text-muted); margin-bottom: 8px; font-size: 1.1rem; }
    .hint { font-size: 0.85rem !important; color: var(--color-text-muted) !important; margin-top: 24px !important; }
    .hint kbd { background: var(--color-bg-input); border: 1px solid var(--color-border); padding: 2px 6px; border-radius: 4px; font-family: var(--font-mono); font-size: 0.8em; }
    
    .status-card h3 { margin-bottom: 16px; }
    .status-card p { margin: 8px 0; color: var(--color-text-muted); }
</style>