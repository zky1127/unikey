use super::*;
use super::ChatRequest;
use super::MessageContent;

pub struct AnthropicProvider {
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat(&self, request: &ChatRequest, api_key: &str) -> Result<ChatResponse, String> {
        let model = request.model.clone().unwrap_or_else(|| "claude-sonnet-5-20251001".into());
        let url = "https://api.anthropic.com/v1/messages";

        // 转换 OpenAI messages → Anthropic messages
        let (system, messages) = convert_messages(&request.messages);

        let mut payload = serde_json::json!({
            "model": model,
            "max_tokens": request.max_tokens,
            "messages": messages,
        });

        if let Some(sys) = system {
            payload["system"] = serde_json::Value::String(sys);
        }
        if request.temperature > 0.0 {
            payload["temperature"] = serde_json::json!(request.temperature);
        }
        if request.top_p < 1.0 {
            payload["top_p"] = serde_json::json!(request.top_p);
        }

        let resp = self
            .client
            .post(url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Anthropic request failed: {}", e))?;

        if !resp.status().is_success() {
            let err_body = resp.text().await.unwrap_or_default();
            return Err(format!("Anthropic API error: {}", err_body));
        }

        let body: serde_json::Value =
            resp.json().await.map_err(|e| format!("Anthropic parse error: {}", e))?;

        convert_response(&body)
    }

    async fn chat_stream(
        &self,
        _request: &ChatRequest,
        _api_key: &str,
    ) -> Result<
        Box<dyn futures::Stream<Item = Result<StreamChunk, String>> + Unpin + Send>,
        String,
    > {
        Err("Anthropic streaming not yet implemented".to_string())
    }
}

/// 提取 system 消息，构建 Anthropic 格式 messages
fn convert_messages(messages: &[Message]) -> (Option<String>, Vec<serde_json::Value>) {
    let mut system = None;
    let mut anthropic_msgs = Vec::new();

    for msg in messages {
        let content_str = match &msg.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join("\n"),
        };

        if msg.role == "system" {
            system = Some(content_str);
        } else {
            let role = match msg.role.as_str() {
                "assistant" => "assistant",
                _ => "user",
            };
            anthropic_msgs.push(serde_json::json!({
                "role": role,
                "content": content_str,
            }));
        }
    }

    (system, anthropic_msgs)
}

/// 转换 Anthropic 响应 → OpenAI 格式
fn convert_response(body: &serde_json::Value) -> Result<ChatResponse, String> {
    let content = body["content"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    let input_tokens = body["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
    let output_tokens = body["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;

    Ok(ChatResponse {
        id: body["id"].as_str().unwrap_or("").to_string(),
        object: "chat.completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model: body["model"].as_str().unwrap_or("").to_string(),
        choices: vec![Choice {
            index: 0,
            message: Message {
                role: "assistant".to_string(),
                content: MessageContent::Text(content),
            },
            finish_reason: Some(body["stop_reason"].as_str().unwrap_or("stop").to_string()),
        }],
        usage: Usage {
            prompt_tokens: input_tokens,
            completion_tokens: output_tokens,
            total_tokens: input_tokens + output_tokens,
        },
    })
}
