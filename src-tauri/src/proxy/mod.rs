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
use tokio::sync::Mutex;

use crate::providers::*;
use crate::storage::Storage;

use router::Router as UniKeyRouter;
use format::FormatTranslator;

/// 代理服务器共享状态
pub struct ProxyState {
    pub storage: Arc<Storage>,
    pub registry: Arc<ProviderRegistry>,
    pub router: Arc<Mutex<UniKeyRouter>>,
    pub translator: Arc<FormatTranslator>,
}

/// 启动代理服务器
pub async fn start_proxy(
    port: u16,
    storage: Arc<Storage>,
    registry: Arc<ProviderRegistry>,
) -> Result<tokio::task::JoinHandle<()>, String> {
    let router = Arc::new(Mutex::new(UniKeyRouter::new()));
    let translator = Arc::new(FormatTranslator::new());

    let state = Arc::new(ProxyState {
        storage,
        registry,
        router,
        translator,
    });

    let app = Router::new()
        // OpenAI 兼容端点
        .route("/v1/chat/completions", post(chat_completions))
        // Anthropic 兼容端点
        .route("/v1/messages", post(anthropic_messages))
        // 健康检查
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

    // 2. 验证并获取场景配置
    let scene = state
        .router
        .lock()
        .await
        .resolve(&state.storage, &unified_key)
        .await
        .map_err(AppError::internal)?;

    // 3. 解析请求
    let request: ChatRequest =
        serde_json::from_str(&body).map_err(|e| AppError::bad_request(e.to_string()))?;

    // 4. 路由决策：选择 provider + 模型
    let resolution = state
        .router
        .lock()
        .await
        .decide(&scene, &request)
        .await
        .map_err(AppError::internal)?;

    // 5. 获取 Provider
    let provider = state
        .registry
        .get(&resolution.provider)
        .ok_or_else(|| AppError::internal(format!("Unknown provider: {}", resolution.provider)))?;

    // 6. 翻译请求格式
    let translated_request = state
        .translator
        .translate_request(&request, &resolution.target_format);

    // 7. 调用真实 API
    let start = std::time::Instant::now();
    let result = provider.chat(&translated_request, &resolution.api_key).await;
    let _latency = start.elapsed().as_millis() as u64;

    // 8. 记录日志
    // TODO: log to storage

    // 9. 翻译响应回 OpenAI 格式
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

    let scene = state
        .router
        .lock()
        .await
        .resolve(&state.storage, &unified_key)
        .await
        .map_err(AppError::internal)?;

    // 解析 Anthropic 请求
    let anthropic_req: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| AppError::bad_request(e.to_string()))?;

    // 转换为内部格式
    let request = state
        .translator
        .anthropic_to_internal(&anthropic_req);

    let resolution = state
        .router
        .lock()
        .await
        .decide(&scene, &request)
        .await
        .map_err(AppError::internal)?;

    let provider = state
        .registry
        .get(&resolution.provider)
        .ok_or_else(|| AppError::internal(format!("Unknown provider: {}", resolution.provider)))?;

    let translated_request = state
        .translator
        .translate_request(&request, &resolution.target_format);

    let result = provider.chat(&translated_request, &resolution.api_key).await;

    match result {
        Ok(response) => {
            // 转换回 Anthropic 格式
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

/// 从 OpenAI 格式请求头提取统一 Key
fn extract_unified_key(headers: &HeaderMap) -> Result<String, AppError> {
    let auth = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorized("Missing Authorization header"))?;

    auth.strip_prefix("Bearer ")
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::unauthorized("Invalid Authorization format"))
}

/// 从 Anthropic 格式请求头提取统一 Key
fn extract_unified_key_anthropic(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::unauthorized("Missing x-api-key header"))
}

// ========== Error Handling ==========

struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    fn bad_request(msg: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg,
        }
    }
    fn unauthorized(msg: &str) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: msg.to_string(),
        }
    }
    fn internal(msg: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg,
        }
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
