# Contributing to UniKey

欢迎贡献！无论你是发现 bug、提 feature request、还是提交代码，都非常感谢。

## 快速开始

```bash
# 环境要求
# - Rust (https://rustup.rs)
# - Node.js 22+
# - Windows: Visual Studio Build Tools
# - macOS: Xcode Command Line Tools
# - Linux: libwebkit2gtk-4.1-dev, libgtk-3-dev, etc.

# 克隆 & 运行
git clone https://github.com/zky1127/unikey.git
cd unikey
npm install
npm run tauri dev
```

## 项目结构

```
src/                    # React 前端
├── App.tsx             # 主组件 (5 个页面)
├── App.css             # 暗色主题样式
└── lib/api.ts          # Tauri IPC API 封装

src-tauri/src/          # Rust 后端
├── main.rs             # 入口
├── lib.rs              # Tauri Commands + 测试
├── storage/            # SQLite + AES-256 加密
├── providers/          # AI 厂商适配器
│   ├── mod.rs          # Provider trait + 注册表 + 流式辅助
│   ├── openai.rs       # OpenAI
│   ├── anthropic.rs    # Anthropic (含格式互转)
│   ├── deepseek.rs     # DeepSeek
│   └── qwen.rs         # 通义千问/智谱/Kimi/Gemini/百川/豆包/MiniMax/Ollama
├── proxy/              # HTTP 代理服务器
│   ├── mod.rs          # Axum 路由 + 流式/非流式
│   ├── router.rs       # 统一 Key → 场景 → 路由决策
│   └── format.rs       # 格式翻译 (OpenAI ↔ Anthropic)
└── recommend.rs        # 智能推荐引擎 + 预设方案库
```

## 如何添加新的 AI Provider

1. 在 `src-tauri/src/providers/qwen.rs` 中用 `impl_openai_compatible!` 宏添加：

```rust
impl_openai_compatible!(XxxProvider, "xxx", "model-name",
    "https://api.xxx.com/v1/chat/completions");
```

2. 在 `src-tauri/src/providers/mod.rs` 的 `ProviderRegistry::new()` 中注册：

```rust
registry.register("xxx", Arc::new(qwen::XxxProvider::new()));
```

3. 在前端 `src/App.tsx` 的 `PROVIDERS` 数组中添加

4. 在 `src-tauri/src/proxy/router.rs` 的 `derive_provider()` 中添加映射

## 测试

```bash
cd src-tauri
cargo test          # 运行所有 Rust 测试
cd ..
npx tsc --noEmit    # 前端类型检查
```

## 提交规范

- `feat:` 新功能
- `fix:` 修复
- `docs:` 文档
- `test:` 测试
- `refactor:` 重构

## License

MIT — 贡献即同意 MIT 授权。
