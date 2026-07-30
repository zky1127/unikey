use super::*;
use crate::providers::ChatRequest;

pub struct DeepSeekProvider {
    client: reqwest::Client,
}

impl DeepSeekProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl Provider for DeepSeekProvider {
    fn name(&self) -> &str {
        "deepseek"
    }

    async fn chat(
        &self,
        request: &ChatRequest,
        api_key: &str,
        base_url: Option<&str>,
    ) -> Result<ChatResponse, String> {
        let model = request.model.clone().unwrap_or_else(|| "deepseek-chat".into());
        let url = base_url
            .map(|u| format!("{}/chat/completions", u.trim_end_matches('/')))
            .unwrap_or_else(|| "https://api.deepseek.com/v1/chat/completions".to_string());

        let payload = serde_json::json!({
            "model": model,
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": match &m.content {
                        MessageContent::Text(t) => serde_json::Value::String(t.clone()),
                        MessageContent::Parts(parts) => {
                            serde_json::Value::String(
                                parts.iter()
                                    .filter_map(|p| p.text.clone())
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            )
                        }
                    },
                })
            }).collect::<Vec<_>>(),
            "temperature": request.temperature,
            "top_p": request.top_p,
            "max_tokens": request.max_tokens,
            "frequency_penalty": request.frequency_penalty,
            "presence_penalty": request.presence_penalty,
            "stream": false,
        });

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("DeepSeek request failed: {}", e))?;

        if !resp.status().is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(format!("DeepSeek API error: {}", err_body));
        }

        resp.json::<ChatResponse>()
            .await
            .map_err(|e| format!("DeepSeek response parse error: {}", e))
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
        api_key: &str,
        base_url: Option<&str>,
    ) -> Result<
        Box<dyn futures::Stream<Item = Result<StreamChunk, String>> + Unpin + Send>,
        String,
    > {
        let url = base_url
            .map(|u| format!("{}/chat/completions", u.trim_end_matches('/')))
            .unwrap_or_else(|| "https://api.deepseek.com/v1/chat/completions".to_string());
        let model = request.model.clone().unwrap_or_else(|| "deepseek-chat".into());
        super::stream_openai_compatible(&self.client, &url, api_key, &model, request).await
    }
}
