# Echo 项目计划书与进度报告

- **项目名称**：Echo — AI 桌面/移动端电子宠物
- **代码仓库**：`git@github.com:yijuntuqi/Echo.git`
- **当前分支**：`feature/chat-pipeline-tasks4-8`（Task 4-7 + 交互修复/SenseNova 切换已提交至 `a9f9a70`；`main` 停在 `a1f2a60`，合并/推送待定）
- **技术栈**：Tauri 2 + Rust + Svelte 5 + Vite 6 + SQLCipher + sqlx + Candle
- **目标平台**：Windows 10/11 (EXE/MSI)、Android 8+ (APK/AAB)

## 1. 项目概述

Echo 是一只住在用户桌面/手机桌面上的 AI 电子宠物。它会：

- **陪伴**：常驻桌面透明窗口，点击弹出对话面板
- **记忆**：记住用户的昵称、生日、每日心情、重要大事记
- **成长**：根据交互频次、情绪倾向、时间流逝自动进化（蛋 → 幼年 → 少年 → 成年 → 究极）
- **关怀**：生日/周年自动弹窗祝福，每晚生成「今日回顾」，每周自动加密备份
- **隐私**：纯本地运行，除 LLM 调用外无任何网络请求；用户可选填自己的 API Key，官方共享额度每日 200 次

**核心差异化**：不做 SaaS，不存用户数据，分发仅需一个官网下载链接 + 赞赏码。

## 2. 架构决策记录

| 决策点 | 选择 | 理由 |
| --- | --- | --- |
| 应用框架 | Tauri 2 | 单代码库双端、Rust 核心安全高效、二进制极小 (~10MB) |
| 前端框架 | Svelte 5 (runes) + Vite 6 | 无虚拟 DOM 开销、响应式简洁、体积小 |
| UI 原语 | 自建 Button/TextField/Toggle | bits-ui v2 导入路径变动且体积大，自建更可控 |
| 路由 | 60 行 Hash Router (`$lib/router.ts`) | 仅 4 个视图，无需 SvelteKit/SSR |
| 数据库 | SQLCipher + sqlx | 纯本地、AES-256 加密、单文件、零配置 |
| 向量检索 | sqlite-vec（运行时探测） | 无外部服务、桌面自带、移动端优雅降级 FTS5 |
| Embedding | Candle + bge-small-zh-v1.5 (384 维, 33MB) | 纯 Rust 推理、CPU <50ms、模型随应用打包 |
| LLM 客户端 | reqwest + OpenAI 兼容协议 | 支持流式、结构化输出、指数退避重试、额度感知 |
| 进化引擎 | 状态机 + 32 维性格向量 + 事件溯源 | 可回放、可迁移、可扩展触发器 |
| 定时/后台 | tokio-cron-scheduler + 系统通知 | 系统级定时、生日/周年/每日回顾自动触发 |
| 打包分发 | cargo-bundle (NSIS/MSI) + tauri-cli (APK/AAB) | 原生安装体验、自动更新、代码签名就绪 |
| 密钥管理 | keyring (Windows Credential Manager / macOS Keychain / libsecret) | 替代 tauri-plugin-stronghold，无需编译 libsodium，零 C 依赖 |

## 3. 当前进度总览

### ✅ 已完成（Task 0-3）

| 模块 | 状态 | 关键文件 | 备注 |
| --- | --- | --- | --- |
| 项目脚手架 | ✅ | Cargo.toml / package.json / vite.config.ts / tauri.conf.json | 双入口 (main/pet)、双端配置 |
| 透明宠物窗口 | ✅ | `src-tauri/src/window.rs`、`PetAvatar.svelte` | WS_EX_TRANSPARENT、拖拽、点击穿透切换 |
| 系统托盘 + 热键 | ✅ | `window.rs` | Ctrl+Alt+E、右键菜单、左键切换 |
| 数据库层 | ✅ | `db.rs`、迁移 `1__initial.sql` | SQLCipher、WAL、外键、vec0 运行时探测 |
| 密钥管理 | ✅ | `onboarding.rs` | keyring 生成/存取随机密码，首次运行自动开库 |
| Chat 引擎骨架 | ✅ | `chat.rs` | 流式协议、重试、额度感知、结构化输出 Schema |
| Embedding 服务 | ✅ | `embedding.rs` | Candle + 模型下载器（含进度回调） |
| 向量引擎 | ✅ | `vector.rs` | sqlite-vec + FTS5 双轨、运行时能力探测 |
| 情绪分析 | ✅ | `emotion.rs` | 关键词规则 + LLM 融合策略 |
| 进化引擎 | ✅ | `evolution.rs` | 5 阶段状态机、32 维性格向量、事件溯源 |
| 调度器 | ✅ | `scheduler.rs` | 懒初始化、生日/周年/每日回顾/周备份 |
| 备份/恢复 | ✅ | `backup.rs` | VACUUM INTO 加密快照、自动清理旧备份 |
| 首运向导 | ✅ | `onboarding.rs`、`OnboardingPage.svelte` | 4 步：档案 → 外观 → 通知 → AI Key |
| 前端状态管理 | ✅ | `stores/*.svelte.ts` | Svelte 5 runes、纯对象 store、无 `$` 前缀 |
| 聊天面板 | ✅ | `ChatPanel.svelte` 等 | 流式渲染、Markdown、情绪徽章、输入区 |
| 仪表盘 | ✅ | `EvolutionTree` / `MoodHeatmap` / `Timeline` / `MemoryGraph` | Canvas 可视化、语义搜索 |
| 设置/页面 | ✅ | `SettingsPage` / `HomePage` / `DashboardPage` / `OnboardingPage` | Hash 路由、主题即时生效 |
| 双窗口入口 | ✅ | `index.html` / `pet.html`、`main.ts` / `main-pet.ts` | Vite 多入口、Tauri 双窗口 |
| Task 4: 智谱流式接入 | ✅ (commit `38a798f`) | `chat.rs` | 流式管道、Key 持久化（用户 Key 优先，共享 Key 兜底）；`a9f9a70` 起供应商切至 SenseNova（日常 `sensenova-6.8-flash-lite` / premium `glm-5.2`） |
| Task 5: 模型自动下载 | ✅ (commit `1afa437`) | `embedding.rs` + 前端进度条 | `model:progress` / `model:done` 事件、失败降级 |

### 🚧 进行中 / 待完成

| 任务 | 优先级 | 预估工时 | 依赖 | 备注 |
| --- | --- | --- | --- | --- |
| Task 6: 情绪字段回传闭环 | ✅ (commit `b5362ad`) | `chat.rs` / `emotion.rs` / `App.svelte` | 规则+模型融合分类、conversations/moods 落库、`mood:updated` 实时热力图；附带修好 SQLCipher key 语法（`764fbb9`） |
| Task 7: 进化触发闭环 | ✅ (本轮提交) | `evolution.rs` / `scheduler.rs` / `chat.rs` | 每轮对话 + 每日 09:10 双触发 `evaluate`、推送 `evolution:triggered`、双窗口动画；`get_evolution_state` 带实时 progress |
| Task 8: 生日/周年/每日回顾内容生成 | 🟢 低 | 1 天 | Task 4 | 调用 premium 模型生成回顾卡片、写 events、系统通知；调度器任务位已就绪（现为日志占位） |
| Task 9: Windows 代码签名 & 自动更新 | 🟢 低 | 1 天 | 证书 | NSIS/MSI 签名、GitHub Releases 更新端点 |
| Task 10: Android 打包 & 预编译 sqlite-vec | 🟢 低 | 2 天 | NDK 环境 | aarch64 预编译 .so、APK/AAB 签名 |
| Task 11: 宠物 SVG 素材制作 | 🟡 中 | 1-2 天 | 设计 | 5 阶段 × 6 状态 = 30 个 SVG (200×200) |
| Task 12: 端到端测试 & 发布 | 🟢 低 | 1 天 | 全部 | Playwright E2E、GitHub Release、官网单页 |

## 4. 运行 / 开发指南

### 环境要求

```bash
# 必须
Node.js 20+ (已有 v24.18.0)
Rust 1.80+ (已有)
pnpm / npm (已有 npm 11.16.0)

# Windows 打包
Visual Studio 2022 + Windows 10 SDK

# Android 打包（可选）
Android SDK / NDK r26+
Java 17+
```

### 启动开发模式

```bash
cd E:\old-new\backup\BNU_leaning\Echo

# 首次安装依赖
npm install

# 启动开发（同时启动 Vite + Tauri）
npx tauri dev
```

### 常用命令

```bash
# 仅前端
npm run dev          # Vite dev server at :1420
npm run build        # TypeScript + Vite build -> dist/
npm run test         # Vitest 单测
npm run test:e2e     # Playwright E2E

# 仅 Rust
cd src-tauri
cargo check          # 快速类型检查
cargo tauri dev      # 等同 npx tauri dev
cargo tauri build    # 释放版构建（需 profile.release）

# 数据库迁移
cd src-tauri
cargo run -- migrate # sqlx migrate run（内嵌 migrate! 宏）

# 清理缓存
cargo clean
rm -rf node_modules dist
npm install
```

### 环境变量（可选）

```bash
# 开发时指定数据库密码（跳过首运向导）
ECHO_DB_PASSWORD=yourpassword npx tauri dev

# 调试日志（代码读取的是 ECHO_LOG，不是 RUST_LOG）
ECHO_LOG=echo=debug,info npx tauri dev

# 强制重新下载模型
ECHO_FORCE_MODEL_DOWNLOAD=1 npx tauri dev
```

## 5. 关键代码位置速查

| 功能 | 后端 (Rust) | 前端 (Svelte) |
| --- | --- | --- |
| 宠物窗口/托盘/热键 | `src-tauri/src/window.rs` | `PetAvatar.svelte`、`PetWindow.svelte` |
| 数据库/迁移 | `db.rs`、`migrations/1__initial.sql` | — |
| 聊天/流式/额度 | `chat.rs` | `ChatPanel.svelte`、`InputArea.svelte`、`StreamRenderer.svelte` |
| Embedding/模型下载 | `embedding.rs` | — |
| 向量搜索/时间线 | `vector.rs` | `MemoryGraph.svelte`、`Timeline.svelte` |
| 情绪分析 | `emotion.rs` | `EmotionBadge.svelte` |
| 进化/状态机 | `evolution.rs` | `EvolutionTree.svelte`、`evolution.svelte.ts` |
| 调度/通知 | `scheduler.rs` | — |
| 备份/恢复 | `backup.rs` | `SettingsPage.svelte` |
| 首运向导/密钥 | `onboarding.rs` | `OnboardingPage.svelte` |
| 全局状态 | `lib.rs` (AppState) | `stores/*.svelte.ts` |
| IPC 命令/事件 | `lib.rs` (invoke_handler) | `$lib/api/commands.ts`、`$lib/api/events.ts` |
| 类型定义 | `db.rs`、`chat.rs` 等 | `$lib/api/types.ts` |
| 路由 | — | `$lib/router.ts`、`App.svelte` |

## 6. 已知坑点 & 避坑指南

| 坑点 | 症状 | 解决方案 |
| --- | --- | --- |
| E 盘编译中途掉线 | `os error 433` / 设备不存在 | `set CARGO_TARGET_DIR=D:\cargo-target\echo` 或把项目移到 D 盘 |
| tauri-plugin-stronghold 编译失败 | libsodium-sys 下载/编译 libsodium 失败 | 已改用 keyring，无需 C 编译 |
| vec0 迁移报错 | sqlite-vec 扩展未加载时创建虚拟表失败 | 已移出迁移，改为运行时 `ensure_vector_table` 探测 |
| `.svelte.ts` 解析失败 | Vite/TS 不识别复合扩展名 | `vite.config.ts` 加 `extensions: ['.svelte.ts', ...]` + tsconfig paths 映射 `$lib/stores/*` |
| `$store` 语法报错 | 我的 store 是普通对象，无 subscribe | 去掉 `$` 前缀，直接读 getter：`chatStore.isStreaming` |
| PetAvatar 用 CSS transform 移动 | 移的是原生窗口，CSS 无效 | 改用 `invoke('set_pet_position', {x, y})` 驱动 OS 窗口 |
| `sqlx::migrate!` 找不到 migrate | 缺 macros feature | `sqlx = { features = ["macros", ...] }` |
| `Shortcut::new` 用 `Key::KeyE` 报错 | v2 API 用 `Code::KeyE` | `use tauri_plugin_global_shortcut::Code` |
| `JobScheduler::new()` panic | 在非 Tokio 上下文调用 | 改为 OnceLock + block_in_place 懒初始化 |
| `npx tauri dev` 卡住 | Vite 未绑定 1420 端口 / 防火墙 | `netstat -ano` |
| vendored OpenSSL 编译失败 | `Can't locate Locale/Maketext/Simple.pm` | 装 Strawberry Perl（C:\Strawberry）；`.cargo/config.toml` 已用 `OPENSSL_SRC_PERL` 钉死路径。注意 cargo 配置按**当前工作目录**向上发现，`--manifest-path` 从别的目录跑不会生效 |
| `PRAGMA key = x'...'` 语法错误 | 开库失败 `near "x'...'": syntax error` | SQLite pragma 值不收 blob 字面量；SQLCipher raw key 必须写成 `"x'...'"`（带双引号，见 db.rs `open_at`）。另：`kdf_iter = 100_000` 的下划线数字同样非法，已删，用 v4 默认 256000 |
| rune 写在普通 `.ts` 里 | 白屏，Console 报 `rune_outside_svelte` | `$state` 等 rune 只能在 `.svelte.ts` / `.svelte` 文件用；`router.ts` 已改名 `router.svelte.ts`，且 tsconfig 需加精确 paths 映射 `$lib/router`（TS 不自动探测 `.svelte.ts`）。`npm run build` 不报这类错，只在运行时炸 |
| 模块顶层 store 里用 `$effect` | 白屏，报 `effect_orphan` | `$effect` 只能在组件初始化或 `$effect.root()` 里调用；模块级 store（chat/pet/mood）的持久化 effect 全部包进 `$effect.root(() => { $effect(() => …) })`。同类错误会一个模块一个模块地连环爆（router 炸时挡住了 chat，chat 炸时挡住 pet/mood），排查时把整条 import 链都扫一遍 |
| 宠物窗口设 `WS_EX_TRANSPARENT` | 宠物不能拖、点了没反应（所有鼠标事件穿透掉） | `build_pet_window` 初始样式只保留 `WS_EX_LAYERED \| WS_EX_TOPMOST`；点击穿透改为运行时由 `set_click_through` 按需开关（前后端默认 false）。注意 ChatPanel 曾在 `$effect` 里 `setClickThrough(!open)`，面板一关就把穿透钉回去——已删 |
| 主窗口 X 直接销毁 webview | 托盘「设置」第二次点了没反应 | `WindowManager::init` 里对主窗口注册 `CloseRequested → prevent_close + hide()`。注意 Tauri 2 的 `AppHandle` 没有 `on_window_event`（只有 `App` 和单个窗口有），要在 `get_webview_window("main")` 上注册并 clone 窗口进闭包 |
| SenseNova 推理模型思考字段 | 流式面板出现乱码思考文本 / 内容被截断 | SenseNova 思考片段在 `delta.reasoning`（智谱叫 `reasoning_content`）；解析器只转发 `delta.content` 天然兼容。`reasoning: {"exclude": true}` 参数被网关忽略，关不掉思考，首字延迟 ~3s 属正常 |

## 7. 下一步行动建议（交接给 Claude Code）

### 立即可做（阻塞最少）

**当前卡点**：等用户本地验证 `a9f9a70`（宠物拖动/点击、托盘设置重开、首运只填 Key、SenseNova 对话出字）。

**Task 8: 生日/周年/每日回顾内容生成** — 调度器任务位已就绪（`scheduler.rs` 中 anniversary-check / daily-recap 仍是日志占位）：

- 复用 `ChatEngine::complete`（Task 6 加的非流式接口，premium 模型）
- 生成回顾卡片写 `events`，走 `notification:show` 或系统通知
- 提醒：工作区 `.cargo/config.toml` 假设 Strawberry Perl 装在 `C:\Strawberry`（已装 ✓）

**运维清理（用户手动）**：`E:\...\Echo\src-tauri\target\` 残留约 17 GB 旧编译产物（现编译走 `D:\cargo-target\echo`），确认后可整个删除。

### 短期（1 周内）

- feature 分支合并回 `main` 并推送（`main` 尚停在 Task 3 之前的 `a1f2a60`）
- Task 9-10: 代码签名 / Android 打包（需证书与 NDK，均依赖用户侧环境）
- 宠物 SVG 素材（可先用占位图，后续替换）

### 中期（发布前）

- Windows 代码签名证书申请 + GitHub Actions 自动构建签名
- Android NDK 编译 sqlite-vec 预编译 .so + APK/AAB 签名
- Playwright E2E 全流程覆盖（首运 → 对话 → 进化 → 备份 → 恢复）

## 8. 文件树速览（核心）

```
Echo/
├── .gitignore
├── index.html                 # 主窗口入口
├── pet.html                   # 宠物窗口入口
├── package.json
├── vite.config.ts             # 双入口、.svelte.ts 解析
├── svelte.config.js           # vitePreprocess + runes
├── tsconfig.json              # $lib/stores/* 映射 .svelte.ts
├── tailwind.config.ts
├── Cargo.toml                 # workspace root（仅根）
├── src/
│   ├── main.ts                # 主窗口挂载 App.svelte
│   ├── main-pet.ts            # 宠物窗口挂载 PetWindow.svelte
│   ├── App.svelte             # 主壳：路由 + ChatPanel
│   ├── PetWindow.svelte       # 宠物窗口根：PetAvatar
│   ├── lib/
│   │   ├── api/
│   │   │   ├── commands.ts    # invoke 封装
│   │   │   ├── events.ts      # listen 封装
│   │   │   └── types.ts       # 共享 TS 类型
│   │   ├── components/
│   │   │   ├── pet/           # PetAvatar, PetWindow
│   │   │   ├── chat/          # ChatPanel, MessageList, StreamRenderer...
│   │   │   ├── dashboard/     # EvolutionTree, MoodHeatmap...
│   │   │   └── ui/            # Button, TextField, Toggle
│   │   ├── stores/            # pet, chat, evolution, mood, memory, settings, system
│   │   ├── router.ts          # 60 行 hash router
│   │   ├── styles/global.css  # CSS 变量主题
│   │   └── utils/env.ts       # isBrowser, isTauri, safeLocalStorage
│   └── pages/                 # HomePage, DashboardPage, SettingsPage, OnboardingPage
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── build.rs
    ├── src/
    │   ├── main.rs            # 仅入口: echo_lib::run()
    │   ├── lib.rs             # 所有模块 + run() + AppState
    │   ├── window.rs          # 透明窗口、托盘、热键、open_chat
    │   ├── db.rs              # SQLCipher、迁移、Repository
    │   ├── chat.rs            # ChatEngine、QuotaTracker、流式
    │   ├── embedding.rs       # Candle + 模型下载器
    │   ├── vector.rs          # sqlite-vec + FTS5 双轨
    │   ├── emotion.rs         # 关键词 + LLM 融合
    │   ├── evolution.rs       # 状态机 + 性格向量 + 溯源
    │   ├── scheduler.rs       # 懒初始化 cron
    │   ├── backup.rs          # VACUUM INTO 加密备份
    │   └── onboarding.rs      # 首运向导、密钥库、自动开库
    ├── migrations/
    │   └── 1__initial.sql
    └── icons/                 # tauri icon 生成的全套图标
```

## 9. 联系 / 继续开发

- 仓库：`git@github.com:yijuntuqi/Echo.git`（已推送最新 `a1f2a60`）
- 下一步建议：从 **Task 4 (ChatAnywhere 接入)** 开始，改 `src-tauri/src/chat.rs` 的 `stream_completion`，约 80 行，前端零改动。
- 运行验证：`npx tauri dev` → 看到桌面宠物 + 主窗口 → 点击宠物弹出对话框 → 发消息 → 等 Task 4 完成后真正得到 AI 回复。
