<div align="center">

# Echo

**一只住在你桌面上的 AI 电子宠物。**

它会聊天、会记事、有情绪，还会随着相处慢慢长大。

[![Release](https://img.shields.io/github/v/release/yijuntuqi/Echo)](https://github.com/yijuntuqi/Echo/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](./LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%2064--bit-blue)](https://github.com/yijuntuqi/Echo/releases)

</div>

## Features / 功能特性

- 🐾 **透明悬浮宠物** — 常驻桌面的透明小家伙，不挡窗口、不碍操作，随时陪伴
- 💬 **点击即聊** — 点一下宠物就弹出聊天窗口，支持 Markdown 渲染
- 🤖 **接入任意 AI** — 支持任何 OpenAI 兼容的云端 API（自填 Base URL + Key），也支持本地 [Ollama](https://ollama.com) 模型
- 🧠 **持久记忆** — 跨会话记住你聊过的事，聊得越久越懂你
- 🎭 **情绪与进化** — 实时情绪识别 + 进化系统，性格随互动慢慢成长
- 🔐 **本地加密存储** — SQLite + SQLCipher 全盘加密，数据只留在你的电脑上
- ⌨️ **系统托盘 + 全局快捷键** — 托盘常驻，`Ctrl+Alt+E` 一键唤出
- 🔄 **自动更新** — 自动检测新版本并下载安装

## Installation / 安装

前往 [Releases](https://github.com/yijuntuqi/Echo/releases) 页面下载最新的 `Echo_x.x.x_x64-setup.exe`，双击安装即可。

> 系统要求：Windows 10 / 11（64 位）

## Quick Start / 快速开始

1. 首次启动会进入**引导向导**，按提示完成初始化
2. 接入 AI（二选一，之后可在设置中随时更换）：
   - **云端 API**：填入任意 OpenAI 兼容服务的 Base URL 和 API Key
   - **本地模型**：本机安装并启动 [Ollama](https://ollama.com) 后直接选用其模型
3. 开始和它聊天吧

> 🔑 API Key 只保存在本地加密数据库中，不会上传到任何第三方。

## Configuration / 配置

| 配置项 | 说明 |
| --- | --- |
| AI 提供商 | 任意 OpenAI 兼容 API（自填 Base URL + Key） |
| 本地模型 | [Ollama](https://ollama.com) |
| 数据位置 | `%APPDATA%\com.yijuntuqi.echo\`（加密数据库、配置与模型缓存） |

## Tech Stack / 技术栈

| 层 | 技术 |
| --- | --- |
| 桌面框架 | Tauri 2 |
| 后端 | Rust |
| 前端 | Svelte 5 + TypeScript + Tailwind CSS |
| 存储 | SQLite（SQLCipher 加密）+ sqlite-vec 向量检索 |
| 分发与更新 | GitHub Releases + Tauri Updater |

## Project Structure / 项目结构

```text
Echo/
├── src/                    # 前端界面（Svelte 5）：主窗口与宠物窗口
├── src-tauri/              # Rust 后端
│   ├── src/                # 聊天、记忆、情绪、进化等核心模块
│   ├── icons/              # 应用图标
│   └── tauri.conf.json     # 窗口、打包与更新配置
└── .github/workflows/      # CI：tag 推送后自动打包发布
```

## Development / 开发

前置要求：Node.js 20+、Rust 1.80+（MSVC 工具链）、Windows 10 / 11

```bash
git clone https://github.com/yijuntuqi/Echo.git
cd Echo
npm install
npm run tauri dev     # 开发模式（热更新）
npx tauri build       # 构建安装包
```

## License / 许可证

本项目以 [MIT](./LICENSE) 许可证开源。

## Buy Me a Coffee / 请我喝杯咖啡

如果 Echo 陪伴过你，欢迎请作者喝杯咖啡 ☕

<p align="center">
  <img src="./wechat_qr.png" alt="微信赞赏" width="240" />
  <img src="./alipay_qr.png" alt="支付宝赞赏" width="240" />
</p>

## Contact / 联系

问题与建议请提交 [GitHub Issues](https://github.com/yijuntuqi/Echo/issues)。
