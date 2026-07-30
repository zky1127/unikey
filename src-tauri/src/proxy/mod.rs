pub mod router;
pub mod format;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use std::sync::Arc;

use crate::providers::*;
use crate::storage::Storage;

use router::Router as UniKeyRouter;
use format::FormatTranslator;

/// 代理服务器共享状态
pub struct ProxyState {
    pub storage: Arc<Storage>,
    pub registry: Arc<ProviderRegistry>,
    pub router: UniKeyRouter,
    pub translator: FormatTranslator,
}

/// 启动代理服务器
pub async fn start_proxy(
    port: u16,
    storage: Arc<Storage>,
    registry: Arc<ProviderRegistry>,
) -> Result<tokio::task::JoinHandle<()>, String> {
    let state = Arc::new(ProxyState {
        storage,
        registry,
        router: UniKeyRouter::new(),
        translator: FormatTranslator::new(),
    });

    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/messages", post(anthropic_messages))
        .route("/health", axum::routing::get(health))
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Cannot bind to {}: {}", addr, e))?;

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    Ok(handle)
}

/// OpenAI 兼容的 /v1/chat/completions
async fn chat_completions(
    State(state): State<Arc<ProxyState>>,
    headers: HeaderMap,
    body: String,
) -> Result<Response, AppError> {
    // 1. 提取统一 Key
    let unified_key = extract_unified_key(&headers)?;

    // 2. 解析场景 — 预加载所有配置 + 解密 Key
    let resolved = state
        .router
        .resolve(&state.storage, &unified_key)
        .map_err(AppError::internal)?;

    // 3. 解析请求
    let mut request: ChatRequest =
        serde_json::from_str(&body).map_err(|e| AppError::bad_request(e.to_string()))?;

    // 4. 路由决策
    let resolution = state
        .router
        .decide(&resolved, &request)
        .map_err(AppError::internal)?;

    // 5. 用配置参数覆盖请求（用户微调的参数生效）
    request.model = Some(resolution.model.clone());
    request.temperature = resolution.temperature;
    request.top_p = resolution.top_p;
    request.max_tokens = resolution.max_tokens;

    // 6. 获取 Provider
    let provider = state
        .registry
        .get(&resolution.provider)
        .ok_or_else(|| AppError::internal(format!("Unknown provider: {}", resolution.provider)))?;

    // 7. 翻译请求格式
    let translated_request = state
        .translator
        .translate_request(&request, &resolution.target_format);

    // 8. 调用真实 API
    let start = std::time::Instant::now();
    let result = provider
        .chat(&translated_request, &resolution.api_key, resolution.base_url.as_deref())
        .await;
    let _latency = start.elapsed().as_millis() as u64;

    // 9. 记录使用日志
    log_proxy_request(
        &state.storage,
        &unified_key,
        &resolution.provider,
        &resolution.model,
        _latency,
        result.is_ok(),
        result.as_ref().err().map(|e| e.as_str()),
    );

    // 10. 翻译响应回 OpenAI 格式
    match result {
        Ok(response) => {
            let final_response = state
                .translator
                .translate_response(&response, &resolution.target_format);
            Ok(Json(final_response).into_response())
        }
        Err(e) => {
            Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "message": e,
                        "type": "unikey_proxy_error"
                    }
                })),
            )
                .into_response())
        }
    }
}

/// Anthropic 兼容的 /v1/messages
async fn anthropic_messages(
    State(state): State<Arc<ProxyState>>,
    headers: HeaderMap,
    body: String,
) -> Result<Response, AppError> {
    let unified_key = extract_unified_key_anthropic(&headers)?;

    let resolved = state
        .router
        .resolve(&state.storage, &unified_key)
        .map_err(AppError::internal)?;

    let anthropic_req: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| AppError::bad_request(e.to_string()))?;

    let mut request = state.translator.anthropic_to_internal(&anthropic_req);

    let resolution = state
        .router
        .decide(&resolved, &request)
        .map_err(AppError::internal)?;

    // Apply config parameters
    request.model = Some(resolution.model.clone());
    request.temperature = resolution.temperature;
    request.top_p = resolution.top_p;
    request.max_tokens = resolution.max_tokens;

    let provider = state
        .registry
        .get(&resolution.provider)
        .ok_or_else(|| AppError::internal(format!("Unknown provider: {}", resolution.provider)))?;

    let translated_request = state
        .translator
        .translate_request(&request, &resolution.target_format);

    let start = std::time::Instant::now();
    let result = provider
        .chat(&translated_request, &resolution.api_key, resolution.base_url.as_deref())
        .await;
    let _latency = start.elapsed().as_millis() as u64;

    log_proxy_request(
        &state.storage,
        &unified_key,
        &resolution.provider,
        &resolution.model,
        _latency,
        result.is_ok(),
        result.as_ref().err().map(|e| e.as_str()),
    );

    match result {
        Ok(response) => {
            let anthropic_response = state.translator.internal_to_anthropic(&response);
            Ok(Json(anthropic_response).into_response())
        }
        Err(e) => {
            Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": {
                        "type": "unikey_proxy_error",
                        "message": e
                    }
                })),
            )
                .into_response())
        }
    }
}

async fn health() -> &'static str {
    "UniKey Proxy Running"
}

// ========== Helpers ==========

fn extract_unified_key(headers: &HeaderMap) -> Result<String, AppError> {
    let auth = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorized("Missing Authorization header"))?;

    auth.strip_prefix("Bearer ")
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::unauthorized("Invalid Authorization format"))
}

fn extract_unified_key_anthropic(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::unauthorized("Missing x-api-key header"))
}

fn log_proxy_request(
    storage: &Storage,
    unified_key: &str,
    provider: &str,
    model: &str,
    latency_ms: u64,
    success: bool,
    error: Option<&str>,
) {
    use crate::storage::models::ProxyLog;
    let log = ProxyLog {
        id: uuid::Uuid::new_v4().to_string(),
        unified_key_id: unified_key.to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        provider: provider.to_string(),
        model: model.to_string(),
        input_tokens: None,
        output_tokens: None,
        latency_ms,
        success,
        error: error.map(|s| s.to_string()),
    };
    // Non-critical; ignore errors
    let _ = storage.save_proxy_log(&log);
}

// ========== Error Handling ==========

struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn bad_request(msg: String) -> Self {
        Self { status: StatusCode::BAD_REQUEST, message: msg }
    }
    fn unauthorized(msg: &str) -> Self {
        Self { status: StatusCode::UNAUTHORIZED, message: msg.to_string() }
    }
    fn internal(msg: String) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: msg }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({
                "error": {
                    "message": self.message,
                    "type": "unikey_proxy_error"
                }
            })),
        )
            .into_response()
    }
}
