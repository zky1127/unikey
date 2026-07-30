use super::*;
use futures::Stream;

macro_rules! impl_openai_compatible {
    ($name:ident, $label:expr, $default_model:expr, $default_url:expr) => {
        pub struct $name { client: reqwest::Client }

        impl $name {
            pub fn new() -> Self { Self { client: reqwest::Client::new() } }
        }

        #[async_trait::async_trait]
        impl Provider for $name {
            fn name(&self) -> &str { $label }

            async fn chat(
                &self, request: &ChatRequest, api_key: &str, base_url: Option<&str>,
            ) -> Result<ChatResponse, String> {
                let model = request.model.clone().unwrap_or_else(|| $default_model.into());
                let url = base_url
                    .map(|u| format!("{}/chat/completions", u.trim_end_matches('/')))
                    .unwrap_or_else(|| $default_url.to_string());
                call_openai_compatible_chat(&self.client, &url, api_key, &model, request).await
            }

            async fn chat_stream(
                &self, request: &ChatRequest, api_key: &str, base_url: Option<&str>,
            ) -> Result<Box<dyn Stream<Item = Result<StreamChunk, String>> + Unpin + Send>, String> {
                let model = request.model.clone().unwrap_or_else(|| $default_model.into());
                let url = base_url
                    .map(|u| format!("{}/chat/completions", u.trim_end_matches('/')))
                    .unwrap_or_else(|| $default_url.to_string());
                super::stream_openai_compatible(&self.client, &url, api_key, &model, request).await
            }
        }
    };
}

// ====== Provider Definitions ======

impl_openai_compatible!(QwenProvider, "qwen", "qwen-plus",
    "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions");

impl_openai_compatible!(ZhipuProvider, "zhipu", "glm-4-plus",
    "https://open.bigmodel.cn/api/paas/v4/chat/completions");

impl_openai_compatible!(MoonshotProvider, "moonshot", "moonshot-v1-8k",
    "https://api.moonshot.cn/v1/chat/completions");

impl_openai_compatible!(GeminiProvider, "gemini", "gemini-2.5-flash",
    "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions");

impl_openai_compatible!(BaichuanProvider, "baichuan", "baichuan4",
    "https://api.baichuan-ai.com/v1/chat/completions");

impl_openai_compatible!(DoubaoProvider, "doubao", "doubao-pro-32k",
    "https://ark.cn-beijing.volces.com/api/v3/chat/completions");

impl_openai_compatible!(MinimaxProvider, "minimax", "abab6.5s-chat",
    "https://api.minimax.chat/v1/text/chatcompletion_v2");

impl_openai_compatible!(OllamaProvider, "ollama", "llama3",
    "http://localhost:11434/v1/chat/completions");

// ====== Shared chat helper (non-streaming) ======

async fn call_openai_compatible_chat(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    model: &str,
    request: &ChatRequest,
) -> Result<ChatResponse, String> {
    let payload = serde_json::json!({
        "model": model,
        "messages": request.messages.iter().map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": match &m.content {
                    MessageContent::Text(t) => serde_json::Value::String(t.clone()),
                    MessageContent::Parts(parts) => {
                        serde_json::Value::String(
                            parts.iter().filter_map(|p| p.text.clone()).collect::<Vec<_>>().join("\n")
                        )
                    }
                },
            })
        }).collect::<Vec<_>>(),
        "temperature": request.temperature,
        "top_p": request.top_p,
        "max_tokens": request.max_tokens,
        "stream": false,
    });

    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return Err(format!("API error ({}): {}", status, err_body));
    }

    resp.json::<ChatResponse>()
        .await
        .map_err(|e| format!("Response parse error: {}", e))
}
