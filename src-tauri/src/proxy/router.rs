use crate::providers::ChatRequest;
use crate::storage::{models::*, Storage};
use std::collections::HashMap;

/// 路由决策结果
#[derive(Debug, Clone)]
pub struct RouteResolution {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub target_format: String,
    pub base_url: Option<String>,
    pub temperature: f64,
    pub top_p: f64,
    pub max_tokens: u32,
}

/// 解析后的场景（含所有关联的配置和 Key）
pub struct ResolvedScene {
    pub scene: Scene,
    pub configs: HashMap<String, ModelConfig>,
    pub api_keys: HashMap<String, (String, Option<String>)>, // config_id → (decrypted_key, base_url)
}

/// 请求路由器
pub struct Router;

impl Router {
    pub fn new() -> Self {
        Self
    }

    /// 根据统一 Key 解析场景 + 预加载所有关联数据
    pub fn resolve(&self, storage: &Storage, unified_key: &str) -> Result<ResolvedScene, String> {
        let uk = storage
            .get_unified_key(unified_key)?
            .ok_or_else(|| format!("Invalid unified key: {}", unified_key))?;

        let scenes = storage.list_scenes()?;
        let scene = scenes
            .into_iter()
            .find(|s| s.id == uk.scene_id)
            .ok_or_else(|| format!("Scene not found for key: {}", unified_key))?;

        // 预加载场景中用到的所有 model_config
        let all_configs = storage.list_model_configs()?;
        let mut configs: HashMap<String, ModelConfig> = HashMap::new();
        let mut api_keys: HashMap<String, (String, Option<String>)> = HashMap::new();

        for rule in &scene.rules {
            let config_id = &rule.model_config_id;
            if let Some(config) = all_configs.iter().find(|c| c.id == *config_id) {
                configs.insert(config_id.clone(), config.clone());

                // 加载并解密对应的 ProviderKey
                if !api_keys.contains_key(config_id) {
                    let provider_keys = storage.list_provider_keys()?;
                    if let Some(pk) = provider_keys.iter().find(|k| k.id == config.provider_key_id) {
                        let decrypted = storage.decrypt(&pk.encrypted_key)?;
                        api_keys.insert(config_id.clone(), (decrypted, pk.base_url.clone()));
                    }
                }
            }
        }

        // 记录使用
        let _ = storage.record_usage(unified_key);

        Ok(ResolvedScene {
            scene,
            configs,
            api_keys,
        })
    }

    /// 根据场景规则决定路由目标
    pub fn decide(
        &self,
        resolved: &ResolvedScene,
        request: &ChatRequest,
    ) -> Result<RouteResolution, String> {
        let mut sorted_rules: Vec<&RouteRule> = resolved.scene.rules.iter().collect();
        sorted_rules.sort_by_key(|r| r.priority);

        for rule in &sorted_rules {
            if self.match_condition(&rule.condition, request) {
                let config_id = &rule.model_config_id;

                let config = resolved
                    .configs
                    .get(config_id)
                    .ok_or_else(|| format!("ModelConfig not found: {}", config_id))?;

                let (api_key, base_url) = resolved
                    .api_keys
                    .get(config_id)
                    .ok_or_else(|| format!("API key not found for config: {}", config_id))?;

                let provider_key = derive_provider(&config.model);

                return Ok(RouteResolution {
                    provider: provider_key.to_string(),
                    model: config.model.clone(),
                    api_key: api_key.clone(),
                    target_format: provider_to_format(provider_key).to_string(),
                    base_url: base_url.clone(),
                    temperature: config.temperature,
                    top_p: config.top_p,
                    max_tokens: config.max_tokens,
                });
            }
        }

        Err("No matching route rule found for this request".to_string())
    }

    /// 检查请求是否匹配路由条件
    fn match_condition(&self, condition: &RouteCondition, request: &ChatRequest) -> bool {
        match condition {
            RouteCondition::Default => true,
            RouteCondition::Always => true,
            RouteCondition::Keyword { keywords } => {
                let content = extract_request_text(request);
                let content_lower = content.to_lowercase();
                keywords
                    .iter()
                    .any(|kw| content_lower.contains(&kw.to_lowercase()))
            }
            RouteCondition::ModelName { patterns } => {
                if let Some(model) = &request.model {
                    patterns
                        .iter()
                        .any(|p| model.to_lowercase().contains(&p.to_lowercase()))
                } else {
                    false
                }
            }
            RouteCondition::Capability { capabilities } => {
                let has_image = request.messages.iter().any(|m| match &m.content {
                    crate::providers::MessageContent::Parts(parts) => {
                        parts.iter().any(|p| p.image_url.is_some())
                    }
                    _ => false,
                });

                use ModelCapability::*;
                for cap in capabilities {
                    match cap {
                        ImageUnderstanding if has_image => return true,
                        CodeGeneration => {
                            let text = extract_request_text(request).to_lowercase();
                            let code_kw = [
                                "code", "function", "class", "def ", "fn ",
                                "代码", "编程", "实现", "写一个", "debug",
                            ];
                            if code_kw.iter().any(|kw| text.contains(kw)) {
                                return true;
                            }
                        }
                        Translation => {
                            let text = extract_request_text(request).to_lowercase();
                            let trans_kw = [
                                "translate", "翻译", "译成", "convert to english", "翻译成",
                            ];
                            if trans_kw.iter().any(|kw| text.contains(kw)) {
                                return true;
                            }
                        }
                        _ => {}
                    }
                }
                false
            }
        }
    }
}

/// 根据模型名推断 provider
fn derive_provider(model: &str) -> &str {
    let model_lower = model.to_lowercase();
    if model_lower.contains("gpt") || model_lower.contains("o1") || model_lower.contains("o3") || model_lower.contains("o4") {
        "openai"
    } else if model_lower.contains("claude") {
        "anthropic"
    } else if model_lower.contains("deepseek") {
        "deepseek"
    } else if model_lower.contains("qwen") || model_lower.contains("tongyi") {
        "qwen"
    } else if model_lower.contains("glm") {
        "zhipu"
    } else if model_lower.contains("moonshot") || model_lower.contains("kimi") {
        "moonshot"
    } else if model_lower.contains("gemini") {
        "gemini"
    } else {
        "openai" // 默认按 OpenAI 兼容格式处理
    }
}

/// Provider → API 格式
fn provider_to_format(provider: &str) -> &str {
    match provider {
        "anthropic" => "anthropic",
        _ => "openai",
    }
}

/// 提取请求中的所有文本内容（用于关键词匹配）
fn extract_request_text(request: &ChatRequest) -> String {
    request
        .messages
        .iter()
        .map(|m| match &m.content {
            crate::providers::MessageContent::Text(t) => t.clone(),
            crate::providers::MessageContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join(" "),
        })
        .collect::<Vec<_>>()
        .join(" ")
}
