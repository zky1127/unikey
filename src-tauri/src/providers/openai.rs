use super::*;
use crate::providers::ChatRequest;

pub struct OpenAIProvider {
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn chat(&self, request: &ChatRequest, api_key: &str) -> Result<ChatResponse, String> {
        let model = request.model.clone().unwrap_or_else(|| "gpt-4o".into());
        let url = "https://api.openai.com/v1/chat/completions";

        let payload = serde_json::json!({
            "model": model,
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": message_content_to_json(&m.content),
                })
            }).collect::<Vec<_>>(),
            "temperature": request.temperature,
            "top_p": request.top_p,
            "max_tokens": request.max_tokens,
            "frequency_penalty": request.frequency_penalty,
            "presence_penalty": request.presence_penalty,
        });

        let resp = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("OpenAI request failed: {}", e))?;

        if !resp.status().is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(format!("OpenAI API error: {}", err_body));
        }

        resp.json::<ChatResponse>()
            .await
            .map_err(|e| format!("OpenAI response parse error: {}", e))
    }

    async fn chat_stream(
        &self,
        _request: &ChatRequest,
        _api_key: &str,
    ) -> Result<
        Box<dyn futures::Stream<Item = Result<StreamChunk, String>> + Unpin + Send>,
        String,
    > {
        Err("OpenAI streaming not yet implemented".to_string())
    }
}

fn message_content_to_json(content: &MessageContent) -> serde_json::Value {
    match content {
        MessageContent::Text(text) => serde_json::Value::String(text.clone()),
        MessageContent::Parts(parts) => {
            let items: Vec<serde_json::Value> = parts
                .iter()
                .map(|p| {
                    let mut item = serde_json::json!({"type": p.content_type});
                    if let Some(text) = &p.text {
                        item["text"] = serde_json::Value::String(text.clone());
                    }
                    if let Some(img) = &p.image_url {
                        item["image_url"] = serde_json::json!({"url": img.url});
                    }
                    item
                })
                .collect();
            serde_json::Value::Array(items)
        }
    }
}
