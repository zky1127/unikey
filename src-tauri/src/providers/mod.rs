pub mod openai;
pub mod anthropic;
pub mod deepseek;
pub mod qwen;

use serde::{Deserialize, Serialize};

/// 统一内部请求格式（OpenAI 兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: Option<String>,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub stream: bool,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_top_p")]
    pub top_p: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub frequency_penalty: f64,
    #[serde(default)]
    pub presence_penalty: f64,
}

fn default_temperature() -> f64 { 1.0 }
fn default_top_p() -> f64 { 1.0 }
fn default_max_tokens() -> u32 { 4096 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    pub image_url: Option<ImageUrl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

/// 统一内部响应格式（OpenAI 兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 流式响应块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    pub index: u32,
    pub delta: StreamDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDelta {
    pub role: Option<String>,
    pub content: Option<String>,
}

/// Provider trait — 所有模型厂商实现此 trait
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    /// 调用模型（非流式）
    async fn chat(
        &self,
        request: &ChatRequest,
        api_key: &str,
        base_url: Option<&str>,
    ) -> Result<ChatResponse, String>;
    /// 调用模型（流式）
    async fn chat_stream(
        &self,
        request: &ChatRequest,
        api_key: &str,
        base_url: Option<&str>,
    ) -> Result<
        Box<dyn futures::Stream<Item = Result<StreamChunk, String>> + Unpin + Send>,
        String,
    >;
}

use std::collections::HashMap;
use std::sync::Arc;
use futures::Stream;

/// Provider 注册表
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            providers: HashMap::new(),
        };
        // 注册内置 Provider
        registry.register("openai", Arc::new(openai::OpenAIProvider::new()));
        registry.register("anthropic", Arc::new(anthropic::AnthropicProvider::new()));
        registry.register("deepseek", Arc::new(deepseek::DeepSeekProvider::new()));
        registry.register("qwen", Arc::new(qwen::QwenProvider::new()));
        registry.register("zhipu", Arc::new(qwen::ZhipuProvider::new()));
        registry.register("moonshot", Arc::new(qwen::MoonshotProvider::new()));
        registry.register("gemini", Arc::new(qwen::GeminiProvider::new()));
        registry.register("baichuan", Arc::new(qwen::BaichuanProvider::new()));
        registry.register("doubao", Arc::new(qwen::DoubaoProvider::new()));
        registry.register("minimax", Arc::new(qwen::MinimaxProvider::new()));
        registry.register("ollama", Arc::new(qwen::OllamaProvider::new()));
        registry
    }

    pub fn register(&mut self, name: &str, provider: Arc<dyn Provider>) {
        self.providers.insert(name.to_string(), provider);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(name).cloned()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ====== Shared OpenAI-compatible streaming helper ======

use futures::stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;

pub async fn stream_openai_compatible(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    model: &str,
    request: &ChatRequest,
) -> Result<
    Box<dyn Stream<Item = Result<StreamChunk, String>> + Unpin + Send>,
    String,
> {
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
        "stream": true,
    });

    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Stream request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("API error ({}): {}", status, body));
    }

    let mut byte_stream = resp.bytes_stream();
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<StreamChunk, String>>(64);

    let model_owned = model.to_string();
    tokio::spawn(async move {
        let mut buffer = String::new();
        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                    // Process complete SSE lines
                    while let Some(line_end) = buffer.find('\n') {
                        let line = buffer[..line_end].trim().to_string();
                        buffer = buffer[line_end + 1..].to_string();

                        if line.is_empty() || line.starts_with(':') {
                            continue;
                        }
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                break;
                            }
                            match serde_json::from_str::<serde_json::Value>(data) {
                                Ok(json) => {
                                    let chunk = StreamChunk {
                                        id: json["id"].as_str().unwrap_or("").to_string(),
                                        object: "chat.completion.chunk".to_string(),
                                        created: json["created"].as_u64().unwrap_or(0),
                                        model: model_owned.clone(),
                                        choices: json["choices"].as_array().map(|arr| {
                                            arr.iter().map(|c| StreamChoice {
                                                index: c["index"].as_u64().unwrap_or(0) as u32,
                                                delta: StreamDelta {
                                                    role: c["delta"]["role"].as_str().map(|s| s.to_string()),
                                                    content: c["delta"]["content"].as_str().map(|s| s.to_string()),
                                                },
                                                finish_reason: c["finish_reason"].as_str().map(|s| s.to_string()),
                                            }).collect()
                                        }).unwrap_or_default(),
                                    };
                                    if tx.send(Ok(chunk)).await.is_err() {
                                        return; // receiver dropped
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

    Ok(Box::new(ReceiverStream::new(rx)))
}
