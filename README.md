# UniKey — AI 统一密钥工坊 🔑

**一个 Key 搞定所有 AI 场景**

图形化桌面应用，管理你的 AI API Key，对每个模型精细微调参数，自由组合多个模型配置成"场景"，一键生成统一 Key。

复制到任何软件（Dify、Coze、Claude Code、各种 Agent）就能用，不用再挨个调试。

## 为什么用 UniKey？

| | 传统方式 | UniKey |
|---|---|---|
| 管 Key | 散落在各个平台、各个软件 | **一处管理，加密存储** |
| 调参数 | 每个软件里重复调 | **调一次，到处用** |
| 换模型 | 改配置、找 Key、改地址 | **图形化组合，生成新 Key** |
| 省钱 | 手动判断用哪个 | **智能路由，自动选最合适的** |

## 功能

- 🔑 **密钥管理** — 添加任意厂商 Key，本地 AES-256 加密存储
- 🎛️ **参数微调** — 温度、Top P、Max Tokens、System Prompt 等
- 🔗 **场景组合** — 多模型按规则自动路由（默认/关键词/能力匹配）
- 🚀 **统一 Key** — 一个 Key 代表一个场景，复制即用
- 🌐 **多格式支持** — OpenAI + Anthropic API 兼容
- 📦 **本地代理** — 内置 HTTP 代理，透明转发

## 技术栈

- **桌面框架**：Tauri 2 + Rust
- **前端**：React 19 + TypeScript + Vite
- **代理后端**：Axum + Tokio
- **存储**：SQLite + AES-256-GCM 加密

## 开发

```bash
# 前置条件
# - Rust + Cargo (https://rustup.rs)
# - Node.js 18+
# - Visual Studio Build Tools (Windows)

# 安装依赖
npm install

# 启动开发模式
npm run tauri dev

# 构建发布版
npm run tauri build
```

## License

MIT
