use serde::{Deserialize, Serialize};

/// 加密存储的 API Key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderKey {
    pub id: String,
    pub name: String,           // 用户自定义名称，如 "我的DeepSeek"
    pub provider: String,       // "openai" | "anthropic" | "deepseek" | "custom"
    pub encrypted_key: String,  // AES-256-GCM 加密后的 key
    pub base_url: Option<String>, // 自定义 endpoint
    pub created_at: i64,
}

/// 微调后的模型配置（一个 ProviderKey 可以有多个配置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub provider_key_id: String,  // 关联的 ProviderKey
    pub name: String,             // 配置名称，如 "代码DeepSeek"
    pub model: String,            // 具体模型名，如 "deepseek-chat"
    pub temperature: f64,
    pub top_p: f64,
    pub max_tokens: u32,
    pub frequency_penalty: f64,
    pub presence_penalty: f64,
    pub system_prompt: Option<String>,
    pub extra_params: Option<serde_json::Value>, // 模型特有参数
    pub created_at: i64,
}

/// 场景路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    pub id: String,
    pub condition: RouteCondition,
    pub model_config_id: String,  // 路由到的模型配置
    pub priority: u32,
}

/// 路由条件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteCondition {
    /// 默认路由（兜底）
    Default,
    /// 根据关键词匹配
    Keyword { keywords: Vec<String> },
    /// 根据能力需求匹配
    Capability {
        capabilities: Vec<ModelCapability>,
    },
    /// 根据模型名匹配（如果请求指定了具体模型）
    ModelName { patterns: Vec<String> },
    /// 始终使用（覆盖所有其他规则）
    Always,
}

/// 模型能力标签
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    TextGeneration,
    CodeGeneration,
    CodeReview,
    ImageUnderstanding,
    ImageGeneration,
    Translation,
    Reasoning,
    LongContext,
    ToolUse,
    Embedding,
}

/// 场景 = 一组路由规则的集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub id: String,
    pub name: String,             // 场景名，如 "编程全能"
    pub description: String,
    pub rules: Vec<RouteRule>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 统一 Key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedKey {
    pub id: String,
    pub key_value: String,        // sk-unikey-xxxx
    pub scene_id: String,
    pub name: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub usage_count: u64,
}

/// 代理请求日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyLog {
    pub id: String,
    pub unified_key_id: String,
    pub timestamp: i64,
    pub provider: String,
    pub model: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub latency_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}
