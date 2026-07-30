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

    async fn chat(
        &self,
        request: &ChatRequest,
        api_key: &str,
        base_url: Option<&str>,
    ) -> Result<ChatResponse, String> {
        let model = request.model.clone().unwrap_or_else(|| "claude-sonnet-5-20251001".into());
        let url = base_url
            .map(|u| format!("{}/messages", u.trim_end_matches('/')))
            .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());

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
        request: &ChatRequest,
        api_key: &str,
        base_url: Option<&str>,
    ) -> Result<
        Box<dyn futures::Stream<Item = Result<StreamChunk, String>> + Unpin + Send>,
        String,
    > {
        let model = request.model.clone().unwrap_or_else(|| "claude-sonnet-5-20251001".into());
        let url = base_url
            .map(|u| format!("{}/messages", u.trim_end_matches('/')))
            .unwrap_or_else(|| "https://api.anthropic.com/v1/messages".to_string());

        let (system, messages) = convert_messages(&request.messages);

        let mut payload = serde_json::json!({
            "model": model,
            "max_tokens": request.max_tokens,
            "messages": messages,
            "stream": true,
        });
        if let Some(sys) = system {
            payload["system"] = serde_json::Value::String(sys);
        }
        if request.temperature > 0.0 {
            payload["temperature"] = serde_json::json!(request.temperature);
        }

        let resp = self
            .client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Anthropic stream request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = resp.text().await.unwrap_or_default();
            return Err(format!("Anthropic API error ({}): {}", status, err_body));
        }

        let mut byte_stream = resp.bytes_stream();
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<StreamChunk, String>>(64);
        let model_owned = model.to_string();

        tokio::spawn(async move {
            use futures::StreamExt;
            let mut buffer = String::new();
            let mut msg_id = String::new();
            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].trim().to_string();
                            buffer = buffer[line_end + 1..].to_string();
                            if line.is_empty() { continue; }
                            // Anthropic SSE: "event: type\ndata: {...}"
                            if let Some(data) = line.strip_prefix("data: ") {
                                match serde_json::from_str::<serde_json::Value>(data) {
                                    Ok(json) => {
                                        let event_type = json["type"].as_str().unwrap_or("");
                                        match event_type {
                                            "message_start" => {
                                                msg_id = json["message"]["id"].as_str().unwrap_or("").to_string();
                                            }
                                            "content_block_delta" => {
                                                let text = json["delta"]["text"].as_str().unwrap_or("");
                                                let chunk = StreamChunk {
                                                    id: msg_id.clone(),
                                                    object: "chat.completion.chunk".to_string(),
                                                    created: 0,
                                                    model: model_owned.clone(),
                                                    choices: vec![StreamChoice {
                                                        index: 0,
                                                        delta: StreamDelta {
                                                            role: None,
                                                            content: Some(text.to_string()),
                                                        },
                                                        finish_reason: None,
                                                    }],
                                                };
                                                if tx.send(Ok(chunk)).await.is_err() { return; }
                                            }
                                            "message_delta" => {
                                                let stop_reason = json["delta"]["stop_reason"].as_str().unwrap_or("stop");
                                                let chunk = StreamChunk {
                                                    id: msg_id.clone(),
                                                    object: "chat.completion.chunk".to_string(),
                                                    created: 0,
                                                    model: model_owned.clone(),
                                                    choices: vec![StreamChoice {
                                                        index: 0,
                                                        delta: StreamDelta { role: None, content: None },
                                                        finish_reason: Some(stop_reason.to_string()),
                                                    }],
                                                };
                                                let _ = tx.send(Ok(chunk)).await;
                                            }
                                            "error" => {
                                                let err = json["error"]["message"].as_str().unwrap_or("Unknown error");
                                                let _ = tx.send(Err(err.to_string())).await;
                                                return;
                                            }
                                            _ => {}
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(Err(format!("Parse error: {}", e))).await;
                                        return;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(format!("Stream error: {}", e))).await;
                        return;
                    }
                }
            }
        });

        Ok(Box::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
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
