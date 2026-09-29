// Echo - Main Entry Point
import './lib/styles/global.css';
import { createApp } from './app';

// Tauri 环境检测
declare global {
    interface Window {
        __TAURI__?: any;
    }
}

async function bootstrap() {
    const app = createApp();
    app.mount('#app');
    
    // 如果在 Tauri 中，等待就绪
    if (window.__TAURI__) {
        try {
            await window.__TAURI__.core.invoke('get_onboarding_status');
            // 应用已初始化
        } catch {
            // 忽略错误
        }
    }
}

bootstrap().catch(console.error);