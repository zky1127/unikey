use crate::providers::ChatRequest;

use crate::storage::{
    models::*,
    Storage,
};

/// 路由决策结果
#[derive(Debug, Clone)]
pub struct RouteResolution {
    pub provider: String,        // 目标 provider 名
    pub model: String,           // 目标模型名
    pub api_key: String,         // 解密后的真实 API Key
    pub target_format: String,   // 目标 API 格式 ("openai" | "anthropic")
}

/// 请求路由器
pub struct Router;

impl Router {
    pub fn new() -> Self {
        Self
    }

    /// 根据统一 Key 解析场景
    pub async fn resolve(
        &self,
        storage: &Storage,
        unified_key: &str,
    ) -> Result<Scene, String> {
        let uk = storage
            .get_unified_key(unified_key)?
            .ok_or_else(|| format!("Invalid unified key: {}", unified_key))?;

        let scenes = storage.list_scenes()?;
        let scene = scenes
            .into_iter()
            .find(|s| s.id == uk.scene_id)
            .ok_or_else(|| format!("Scene not found for key: {}", unified_key))?;

        // 记录使用
        let _ = storage.record_usage(unified_key);

        Ok(scene)
    }

    /// 根据场景规则决定路由目标
    pub async fn decide(
        &self,
        scene: &Scene,
        request: &ChatRequest,
    ) -> Result<RouteResolution, String> {
        // 获取所有配置
        let sorted_rules: Vec<&RouteRule> = {
            let mut rules: Vec<&RouteRule> = scene.rules.iter().collect();
            rules.sort_by_key(|r| r.priority);
            rules
        };

        // 按优先级匹配规则
        for rule in &sorted_rules {
            if self.match_condition(&rule.condition, request) {
                // TODO: 从 storage 加载 config + provider_key
                // MVP: 返回占位路由
                return self.build_resolution(rule);
            }
        }

        Err("No matching route rule found".to_string())
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
                // 简单启发式检测
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
                            let code_kw = ["code", "function", "class", "def ", "fn ", "代码", "编程", "实现"];
                            if code_kw.iter().any(|kw| text.contains(kw)) {
                                return true;
                            }
                        }
                        Translation => {
                            let text = extract_request_text(request).to_lowercase();
                            let trans_kw = ["translate", "翻译", "译成", "convert to english", "翻译成"];
                            if trans_kw.iter().any(|kw| text.contains(kw)) {
                                return true;
                            }
                        }
                        _ => {} // 其他能力在后续版本实现更精确的检测
                    }
                }
                false
            }
        }
    }

    fn build_resolution(&self, _rule: &RouteRule) -> Result<RouteResolution, String> {
        // TODO: 从 storage 加载实际配置
        // MVP 阶段返回占位数据
        Ok(RouteResolution {
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            api_key: "placeholder".to_string(),
            target_format: "openai".to_string(),
        })
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
