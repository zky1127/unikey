use super::*;

/// 通义千问 — OpenAI 兼容格式
pub struct QwenProvider {
    client: reqwest::Client,
}

impl QwenProvider {
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }
}

#[async_trait::async_trait]
impl Provider for QwenProvider {
    fn name(&self) -> &str { "qwen" }

    async fn chat(
        &self, request: &ChatRequest, api_key: &str, base_url: Option<&str>,
    ) -> Result<ChatResponse, String> {
        let model = request.model.clone().unwrap_or_else(|| "qwen-plus".into());
        let url = base_url
            .map(|u| format!("{}/chat/completions", u.trim_end_matches('/')))
            .unwrap_or_else(|| "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions".to_string());

        call_openai_compatible(&self.client, &url, api_key, &model, request).await
    }

    async fn chat_stream(
        &self, _request: &ChatRequest, _api_key: &str, _base_url: Option<&str>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamChunk, String>> + Unpin + Send>, String> {
        Err("Streaming not yet implemented".to_string())
    }
}

/// 智谱 GLM — OpenAI 兼容格式
pub struct ZhipuProvider {
    client: reqwest::Client,
}

impl ZhipuProvider {
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }
}

#[async_trait::async_trait]
impl Provider for ZhipuProvider {
    fn name(&self) -> &str { "zhipu" }

    async fn chat(
        &self, request: &ChatRequest, api_key: &str, base_url: Option<&str>,
    ) -> Result<ChatResponse, String> {
        let model = request.model.clone().unwrap_or_else(|| "glm-4-plus".into());
        let url = base_url
            .map(|u| format!("{}/chat/completions", u.trim_end_matches('/')))
            .unwrap_or_else(|| "https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string());

        call_openai_compatible(&self.client, &url, api_key, &model, request).await
    }

    async fn chat_stream(
        &self, _request: &ChatRequest, _api_key: &str, _base_url: Option<&str>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamChunk, String>> + Unpin + Send>, String> {
        Err("Streaming not yet implemented".to_string())
    }
}

/// Kimi (月之暗面) — OpenAI 兼容格式
pub struct MoonshotProvider {
    client: reqwest::Client,
}

impl MoonshotProvider {
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }
}

#[async_trait::async_trait]
impl Provider for MoonshotProvider {
    fn name(&self) -> &str { "moonshot" }

    async fn chat(
        &self, request: &ChatRequest, api_key: &str, base_url: Option<&str>,
    ) -> Result<ChatResponse, String> {
        let model = request.model.clone().unwrap_or_else(|| "moonshot-v1-8k".into());
        let url = base_url
            .map(|u| format!("{}/chat/completions", u.trim_end_matches('/')))
            .unwrap_or_else(|| "https://api.moonshot.cn/v1/chat/completions".to_string());

        call_openai_compatible(&self.client, &url, api_key, &model, request).await
    }

    async fn chat_stream(
        &self, _request: &ChatRequest, _api_key: &str, _base_url: Option<&str>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamChunk, String>> + Unpin + Send>, String> {
        Err("Streaming not yet implemented".to_string())
    }
}

/// Gemini — OpenAI 兼容格式
pub struct GeminiProvider {
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }
}

#[async_trait::async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &str { "gemini" }

    async fn chat(
        &self, request: &ChatRequest, api_key: &str, base_url: Option<&str>,
    ) -> Result<ChatResponse, String> {
        let model = request.model.clone().unwrap_or_else(|| "gemini-2.5-flash".into());
        let url = base_url
            .map(|u| format!("{}/chat/completions", u.trim_end_matches('/')))
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions".to_string());

        call_openai_compatible(&self.client, &url, api_key, &model, request).await
    }

    async fn chat_stream(
        &self, _request: &ChatRequest, _api_key: &str, _base_url: Option<&str>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<StreamChunk, String>> + Unpin + Send>, String> {
        Err("Streaming not yet implemented".to_string())
    }
}

// ====== Shared OpenAI-compatible caller ======

async fn call_openai_compatible(
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
