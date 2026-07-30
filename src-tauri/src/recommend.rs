use serde::{Deserialize, Serialize};

/// 用户需求输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserNeeds {
    pub scenarios: Vec<String>,   // 使用场景: ["编程", "写作", "翻译", "数据分析", "多模态", "日常对话"]
    pub budget: Budget,           // 预算偏好
    pub quality: Quality,         // 质量要求
    pub language: Language,       // 主要语言
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Budget {
    Free,         // 尽量免费
    Balanced,     // 性价比
    Premium,      // 质量优先
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Quality {
    Sufficient,   // 够用就行
    Good,         // 中等
    Best,         // 最好
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    Chinese,
    English,
    Mixed,
}

/// 推荐结果 — 一个完整的场景配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub name: String,
    pub description: String,
    pub estimated_monthly_cost: String,
    pub slots: Vec<RecommendSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendSlot {
    pub role: String,             // 角色名，如 "代码生成"
    pub provider: String,         // 推荐 provider
    pub model: String,            // 推荐模型
    pub temperature: f64,
    pub max_tokens: u32,
    pub reason: String,           // 推荐理由
}

/// 预设场景
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub slots: Vec<RecommendSlot>,
    pub tags: Vec<String>,
}

/// 引擎入口
pub fn recommend(needs: &UserNeeds) -> Vec<Recommendation> {
    let mut results = Vec::new();

    let has_code = needs.scenarios.iter().any(|s| s.contains("编程") || s.contains("代码"));
    let has_write = needs.scenarios.iter().any(|s| s.contains("写作") || s.contains("文案"));
    let has_trans = needs.scenarios.iter().any(|s| s.contains("翻译"));
    let has_data = needs.scenarios.iter().any(|s| s.contains("数据分析") || s.contains("数据"));
    let has_vision = needs.scenarios.iter().any(|s| s.contains("多模态") || s.contains("图片"));
    let has_chat = needs.scenarios.iter().any(|s| s.contains("对话") || s.contains("聊天"));

    let use_premium = matches!(needs.budget, Budget::Premium) || matches!(needs.quality, Quality::Best);

    // Rule-based recommendation logic
    if has_code || has_data {
        let mut slots = Vec::new();
        if has_code {
            slots.push(RecommendSlot {
                role: "代码生成".into(),
                provider: if use_premium { "anthropic" } else { "deepseek" }.into(),
                model: if use_premium { "claude-sonnet-5" } else { "deepseek-chat" }.into(),
                temperature: 0.3,
                max_tokens: 8192,
                reason: if use_premium { "Claude 代码能力业界最强".into() } else { "DeepSeek 代码能力优秀且便宜".into() },
            });
        }
        if has_data {
            slots.push(RecommendSlot {
                role: "数据分析".into(),
                provider: "anthropic".into(),
                model: "claude-sonnet-5".into(),
                temperature: 0.1,
                max_tokens: 16384,
                reason: "Claude 长上下文 + 推理能力适合数据分析".into(),
            });
        }
        results.push(Recommendation {
            name: if has_code && has_data { "编程+数据全能".into() } else if has_code { "编程能手".into() } else { "数据分析专家".into() },
            description: format!("推荐 {} 个模型配置，覆盖{}", slots.len(), if has_code && has_data { "编程和数据分析" } else if has_code { "编程" } else { "数据分析" }),
            estimated_monthly_cost: if use_premium { "~$30-60/月" } else { "~$5-15/月" }.into(),
            slots,
        });
    }

    if has_write || has_trans || has_chat {
        let mut slots = Vec::new();
        if has_write {
            slots.push(RecommendSlot {
                role: "文案创作".into(),
                provider: if use_premium { "openai" } else { "deepseek" }.into(),
                model: if use_premium { "gpt-4o" } else { "deepseek-chat" }.into(),
                temperature: 0.8,
                max_tokens: 4096,
                reason: if use_premium { "GPT-4o 创意写作最强".into() } else { "DeepSeek 中文写作优秀".into() },
            });
        }
        if has_trans {
            slots.push(RecommendSlot {
                role: "翻译".into(),
                provider: "deepseek".into(),
                model: "deepseek-chat".into(),
                temperature: 0.2,
                max_tokens: 4096,
                reason: "DeepSeek 翻译质量好且便宜".into(),
            });
        }
        if has_chat || (!has_code && !has_data) {
            if !slots.iter().any(|s| s.role == "日常对话") {
                slots.push(RecommendSlot {
                    role: "日常对话".into(),
                    provider: "deepseek".into(),
                    model: "deepseek-chat".into(),
                    temperature: 0.7,
                    max_tokens: 4096,
                    reason: "DeepSeek 性价比最高".into(),
                });
            }
        }
        results.push(Recommendation {
            name: "内容创作".into(),
            description: format!("覆盖写作、翻译等场景，{} 个配置", slots.len()),
            estimated_monthly_cost: if use_premium { "~$15-30/月" } else { "~$2-8/月" }.into(),
            slots,
        });
    }

    if has_vision {
        results.push(Recommendation {
            name: "多模态处理".into(),
            description: "图片理解 + 文本生成".into(),
            estimated_monthly_cost: "~$10-25/月".into(),
            slots: vec![RecommendSlot {
                role: "图片理解".into(),
                provider: "openai".into(),
                model: "gpt-4o".into(),
                temperature: 0.5,
                max_tokens: 4096,
                reason: "GPT-4o 多模态能力最成熟".into(),
            }],
        });
    }

    if results.is_empty() {
        results.push(Recommendation {
            name: "通用助手".into(),
            description: "日常 AI 对话".into(),
            estimated_monthly_cost: "免费~$5/月".into(),
            slots: vec![RecommendSlot {
                role: "通用对话".into(),
                provider: "deepseek".into(),
                model: "deepseek-chat".into(),
                temperature: 0.7,
                max_tokens: 4096,
                reason: "DeepSeek 有免费额度，中文能力强".into(),
            }],
        });
    }

    results
}

/// 返回所有内置预设
pub fn get_presets() -> Vec<Preset> {
    vec![
        Preset {
            id: "code-master".into(),
            name: "💻 编程全能".into(),
            description: "代码生成(DeepSeek) + 代码审查(Claude) + 技术问答".into(),
            icon: "💻".into(),
            tags: vec!["编程".into(), "开发".into(), "技术".into()],
            slots: vec![
                RecommendSlot { role: "代码生成".into(), provider: "deepseek".into(), model: "deepseek-chat".into(), temperature: 0.3, max_tokens: 8192, reason: "性价比最高的代码生成".into() },
                RecommendSlot { role: "代码审查".into(), provider: "anthropic".into(), model: "claude-sonnet-5".into(), temperature: 0.1, max_tokens: 16384, reason: "Claude 代码审查最细致".into() },
            ],
        },
        Preset {
            id: "content-creator".into(),
            name: "✍️ 内容创作".into(),
            description: "中文写作(DeepSeek) + 英文润色(GPT) + 翻译".into(),
            icon: "✍️".into(),
            tags: vec!["写作".into(), "自媒体".into(), "营销".into()],
            slots: vec![
                RecommendSlot { role: "中文写作".into(), provider: "deepseek".into(), model: "deepseek-chat".into(), temperature: 0.8, max_tokens: 4096, reason: "中文创作流畅自然".into() },
                RecommendSlot { role: "英文润色".into(), provider: "openai".into(), model: "gpt-4o".into(), temperature: 0.5, max_tokens: 4096, reason: "英文表达最地道".into() },
                RecommendSlot { role: "翻译".into(), provider: "deepseek".into(), model: "deepseek-chat".into(), temperature: 0.2, max_tokens: 4096, reason: "翻译准确且便宜".into() },
            ],
        },
        Preset {
            id: "academic".into(),
            name: "🎓 学术研究".into(),
            description: "深度推理(Claude) + 论文写作(GPT) + 文献分析".into(),
            icon: "🎓".into(),
            tags: vec!["学术".into(), "研究".into(), "论文".into()],
            slots: vec![
                RecommendSlot { role: "深度推理".into(), provider: "anthropic".into(), model: "claude-opus-5".into(), temperature: 0.1, max_tokens: 32768, reason: "最强推理能力".into() },
                RecommendSlot { role: "论文写作".into(), provider: "openai".into(), model: "gpt-4o".into(), temperature: 0.3, max_tokens: 8192, reason: "学术写作规范".into() },
            ],
        },
        Preset {
            id: "multimodal".into(),
            name: "🖼️ 多模态全能".into(),
            description: "图片理解(GPT) + 文本生成(DeepSeek) + 数据分析(Claude)".into(),
            icon: "🖼️".into(),
            tags: vec!["多模态".into(), "设计".into(), "分析".into()],
            slots: vec![
                RecommendSlot { role: "图片理解".into(), provider: "openai".into(), model: "gpt-4o".into(), temperature: 0.5, max_tokens: 4096, reason: "最成熟的多模态支持".into() },
                RecommendSlot { role: "文本生成".into(), provider: "deepseek".into(), model: "deepseek-chat".into(), temperature: 0.7, max_tokens: 4096, reason: "便宜且质量好".into() },
            ],
        },
        Preset {
            id: "budget".into(),
            name: "💰 省钱模式".into(),
            description: "全部路由到 DeepSeek，利用免费额度".into(),
            icon: "💰".into(),
            tags: vec!["省钱".into(), "免费".into(), "入门".into()],
            slots: vec![
                RecommendSlot { role: "默认".into(), provider: "deepseek".into(), model: "deepseek-chat".into(), temperature: 0.7, max_tokens: 4096, reason: "DeepSeek 免费额度".into() },
            ],
        },
        Preset {
            id: "premium".into(),
            name: "👑 最强质量".into(),
            description: "所有场景用最好的模型，不妥协".into(),
            icon: "👑".into(),
            tags: vec!["高品质".into(), "专业".into(), "企业".into()],
            slots: vec![
                RecommendSlot { role: "代码".into(), provider: "anthropic".into(), model: "claude-opus-5".into(), temperature: 0.1, max_tokens: 32768, reason: "最强代码模型".into() },
                RecommendSlot { role: "写作".into(), provider: "openai".into(), model: "gpt-4o".into(), temperature: 0.7, max_tokens: 8192, reason: "最强写作模型".into() },
                RecommendSlot { role: "视觉".into(), provider: "openai".into(), model: "gpt-4o".into(), temperature: 0.3, max_tokens: 4096, reason: "多模态必备".into() },
            ],
        },
        Preset {
            id: "quick-start".into(),
            name: "🚀 快速入门".into(),
            description: "最简单的配置，1 个 DeepSeek Key 搞定所有".into(),
            icon: "🚀".into(),
            tags: vec!["入门".into(), "简单".into(), "新手".into()],
            slots: vec![
                RecommendSlot { role: "默认".into(), provider: "deepseek".into(), model: "deepseek-chat".into(), temperature: 0.7, max_tokens: 4096, reason: "注册就有免费额度".into() },
            ],
        },
    ]
}
