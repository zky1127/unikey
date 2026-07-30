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
        registry.register(
            "anthropic",
            Arc::new(anthropic::AnthropicProvider::new()),
        );
        registry.register("deepseek", Arc::new(deepseek::DeepSeekProvider::new()));
        registry.register("qwen", Arc::new(qwen::QwenProvider::new()));
        registry.register("zhipu", Arc::new(qwen::ZhipuProvider::new()));
        registry.register("moonshot", Arc::new(qwen::MoonshotProvider::new()));
        registry.register("gemini", Arc::new(qwen::GeminiProvider::new()));
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
