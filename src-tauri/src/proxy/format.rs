use crate::providers::*;

/// 格式翻译器 — 在不同 API 格式之间转换
pub struct FormatTranslator;

impl FormatTranslator {
    pub fn new() -> Self {
        Self
    }

    /// 翻译请求到目标格式
    pub fn translate_request(
        &self,
        request: &ChatRequest,
        _target_format: &str,
    ) -> ChatRequest {
        // 目前内部格式就是 OpenAI 格式，转发即可
        // 后续支持 Anthropic → OpenAI 等格式转换
        request.clone()
    }

    /// 翻译响应回 OpenAI 格式
    pub fn translate_response(
        &self,
        response: &ChatResponse,
        _target_format: &str,
    ) -> ChatResponse {
        response.clone()
    }

    /// 将 Anthropic 请求转为内部格式
    pub fn anthropic_to_internal(&self, body: &serde_json::Value) -> ChatRequest {
        let messages: Vec<Message> = body["messages"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|m| {
                        let role = m["role"].as_str().unwrap_or("user").to_string();
                        let content = m["content"]
                            .as_str()
                            .map(|s| MessageContent::Text(s.to_string()))
                            .or_else(|| {
                                m["content"].as_array().map(|parts| {
                                    let content_parts: Vec<ContentPart> = parts
                                        .iter()
                                        .map(|p| ContentPart {
                                            content_type: p["type"]
                                                .as_str()
                                                .unwrap_or("text")
                                                .to_string(),
                                            text: p["text"].as_str().map(|s| s.to_string()),
                                            image_url: p["source"].as_object().and_then(|src| {
                                                src["url"].as_str().map(|url| ImageUrl {
                                                    url: url.to_string(),
                                                })
                                            }),
                                        })
                                        .collect();
                                    MessageContent::Parts(content_parts)
                                })
                            })
                            .unwrap_or(MessageContent::Text(String::new()));
                        Message { role, content }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let system = body["system"].as_str().map(|s| Message {
            role: "system".to_string(),
            content: MessageContent::Text(s.to_string()),
        });

        let mut all_messages = Vec::new();
        if let Some(sys) = system {
            all_messages.push(sys);
        }
        all_messages.extend(messages);

        ChatRequest {
            model: body["model"].as_str().map(|s| s.to_string()),
            messages: all_messages,
            stream: body["stream"].as_bool().unwrap_or(false),
            temperature: body["temperature"].as_f64().unwrap_or(1.0),
            top_p: body["top_p"].as_f64().unwrap_or(1.0),
            max_tokens: body["max_tokens"].as_u64().unwrap_or(4096) as u32,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
        }
    }

    /// 将内部格式转为 Anthropic 响应
    pub fn internal_to_anthropic(&self, response: &ChatResponse) -> serde_json::Value {
        let text = response
            .choices
            .first()
            .map(|c| match &c.message.content {
                MessageContent::Text(t) => t.clone(),
                MessageContent::Parts(parts) => {
                    parts.iter().filter_map(|p| p.text.clone()).collect::<Vec<_>>().join("")
                }
            })
            .unwrap_or_default();

        serde_json::json!({
            "id": response.id,
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": text}],
            "model": response.model,
            "stop_reason": response.choices.first().and_then(|c| c.finish_reason.clone()).unwrap_or_else(|| "end_turn".to_string()),
            "usage": {
                "input_tokens": response.usage.prompt_tokens,
                "output_tokens": response.usage.completion_tokens,
            }
        })
    }
}
