# Changelog

## [0.1.0] — 2026-07-30

### 🎉 首次发布

**核心功能：**
- 🔑 密钥管理：AES-256-GCM 本地加密，支持 12 家 AI 厂商
- 🎛️ 模型微调：Temperature / Top P / Max Tokens / Frequency Penalty / Presence Penalty / System Prompt
- 🔗 场景组合：多模型路由（默认 / 关键词 / 能力匹配 / 模型名）
- 🚀 统一 Key：一键生成，复制到任何 OpenAI/Anthropic 兼容软件使用
- 📡 SSE 流式响应：全部 Provider 支持实时流式输出
- 🧠 智能推荐：根据场景/预算/质量自动推荐模型组合
- 📦 7 个预设方案：编程全能 / 内容创作 / 学术研究 / 多模态全能 / 省钱模式 / 最强质量 / 快速入门

**支持的 AI Provider：**
- OpenAI (GPT-4o, GPT-4.1, o3, o4-mini)
- Anthropic Claude (Opus 5, Sonnet 5, Haiku 4.5)
- DeepSeek (Chat, Reasoner)
- 通义千问 (Max, Plus, Turbo)
- 智谱 GLM (4-Plus, 4-Flash)
- Kimi 月之暗面 (8k, 32k)
- Google Gemini (2.5 Pro, 2.5 Flash)
- 百川 (Baichuan 4, 3-Turbo)
- 豆包 (Doubao Pro 32k)
- MiniMax (abab6.5s)
- Ollama (本地模型)
- 自定义 OpenAI 兼容端点

**技术栈：** Tauri 2 + Rust + React 19 + TypeScript + SQLite + Axum

**平台支持：** Windows (NSIS) · macOS (Universal) · Linux (deb/AppImage)
