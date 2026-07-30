mod proxy;
mod providers;
mod storage;

use providers::ProviderRegistry;
use storage::{models::*, Storage};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

/// 应用全局状态
pub struct AppState {
    pub storage: Arc<Storage>,
    pub registry: Arc<ProviderRegistry>,
    proxy_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

// ============ Tauri Commands ============

// -- Provider Keys --

#[tauri::command]
fn list_provider_keys(state: tauri::State<AppState>) -> Result<Vec<ProviderKey>, String> {
    state.storage.list_provider_keys()
}

#[tauri::command]
fn add_provider_key(
    state: tauri::State<AppState>,
    name: String,
    provider: String,
    api_key: String,
    base_url: Option<String>,
) -> Result<ProviderKey, String> {
    let encrypted = state.storage.encrypt(&api_key)?;
    let key = ProviderKey {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        provider,
        encrypted_key: encrypted,
        base_url,
        created_at: now(),
    };
    state.storage.save_provider_key(&key)?;
    Ok(key)
}

#[tauri::command]
fn delete_provider_key(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.storage.delete_provider_key(&id)
}

#[tauri::command]
fn decrypt_key(state: tauri::State<AppState>, encrypted: String) -> Result<String, String> {
    state.storage.decrypt(&encrypted)
}

// -- Model Configs --

#[tauri::command]
fn list_model_configs(state: tauri::State<AppState>) -> Result<Vec<ModelConfig>, String> {
    state.storage.list_model_configs()
}

#[tauri::command]
fn save_model_config(
    state: tauri::State<AppState>,
    config: ModelConfig,
) -> Result<(), String> {
    state.storage.save_model_config(&config)
}

#[tauri::command]
fn delete_model_config(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.storage.delete_model_config(&id)
}

// -- Scenes --

#[tauri::command]
fn list_scenes(state: tauri::State<AppState>) -> Result<Vec<Scene>, String> {
    state.storage.list_scenes()
}

#[tauri::command]
fn save_scene(state: tauri::State<AppState>, scene: Scene) -> Result<(), String> {
    state.storage.save_scene(&scene)
}

#[tauri::command]
fn delete_scene(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.storage.delete_scene(&id)
}

// -- Unified Keys --

#[tauri::command]
fn list_unified_keys(state: tauri::State<AppState>) -> Result<Vec<UnifiedKey>, String> {
    state.storage.list_unified_keys()
}

#[tauri::command]
fn generate_unified_key(
    state: tauri::State<AppState>,
    scene_id: String,
    name: String,
) -> Result<UnifiedKey, String> {
    let key_value = format!("sk-unikey-{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..32].to_string());
    let key = UnifiedKey {
        id: uuid::Uuid::new_v4().to_string(),
        key_value,
        scene_id,
        name,
        created_at: now(),
        last_used_at: None,
        usage_count: 0,
    };
    state.storage.save_unified_key(&key)?;
    Ok(key)
}

#[tauri::command]
fn delete_unified_key(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.storage.delete_unified_key(&id)
}

// -- Proxy Control --

#[tauri::command]
async fn start_proxy(state: tauri::State<'_, AppState>, port: u16) -> Result<(), String> {
    let handle = proxy::start_proxy(
        port,
        state.storage.clone(),
        state.registry.clone(),
    )
    .await?;
    *state.proxy_handle.lock().await = Some(handle);
    Ok(())
}

#[tauri::command]
async fn stop_proxy(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut handle = state.proxy_handle.lock().await;
    if let Some(h) = handle.take() {
        h.abort();
    }
    Ok(())
}

// ============ App Setup ============

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            std::fs::create_dir_all(&app_dir).ok();

            let db_path = app_dir.join("unikey.db");
            let machine_id = app.config().identifier.clone();
            let storage = Arc::new(Storage::new(db_path, &machine_id)?);

            let registry = Arc::new(ProviderRegistry::new());

            app.manage(AppState {
                storage,
                registry,
                proxy_handle: Mutex::new(None),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_provider_keys,
            add_provider_key,
            delete_provider_key,
            decrypt_key,
            list_model_configs,
            save_model_config,
            delete_model_config,
            list_scenes,
            save_scene,
            delete_scene,
            list_unified_keys,
            generate_unified_key,
            delete_unified_key,
            start_proxy,
            stop_proxy,
        ])
        .run(tauri::generate_context!())
        .expect("error while running UniKey");
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// ============ Integration Tests ============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::ChatRequest;
    use crate::proxy::router::Router;

    fn test_storage() -> Storage {
        Storage::new(
            std::path::PathBuf::from(":memory:"),
            "test-password",
        )
        .unwrap()
    }

    #[test]
    fn test_full_pipeline() {
        let storage = test_storage();

        // 1. Add a provider key
        let pk = ProviderKey {
            id: "pk-1".into(),
            name: "Test Key".into(),
            provider: "openai".into(),
            encrypted_key: storage.encrypt("sk-test-api-key").unwrap(),
            base_url: None,
            created_at: 1,
        };
        storage.save_provider_key(&pk).unwrap();

        let keys = storage.list_provider_keys().unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, "Test Key");

        // 2. Create a model config
        let config = ModelConfig {
            id: "cfg-1".into(),
            provider_key_id: "pk-1".into(),
            name: "Test Config".into(),
            model: "gpt-4o".into(),
            temperature: 0.7,
            top_p: 0.9,
            max_tokens: 2048,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            system_prompt: None,
            extra_params: None,
            created_at: 1,
        };
        storage.save_model_config(&config).unwrap();

        let configs = storage.list_model_configs().unwrap();
        assert_eq!(configs.len(), 1);

        // 3. Create a scene with routing rule
        let scene = Scene {
            id: "scene-1".into(),
            name: "Test Scene".into(),
            description: "Integration test".into(),
            rules: vec![RouteRule {
                id: "rule-1".into(),
                condition: RouteCondition::Always,
                model_config_id: "cfg-1".into(),
                priority: 1,
            }],
            created_at: 1,
            updated_at: 1,
        };
        storage.save_scene(&scene).unwrap();

        // 4. Generate unified key
        let uk = UnifiedKey {
            id: "uk-1".into(),
            key_value: "sk-unikey-testkey000000000000001".into(),
            scene_id: "scene-1".into(),
            name: "Test UK".into(),
            created_at: 1,
            last_used_at: None,
            usage_count: 0,
        };
        storage.save_unified_key(&uk).unwrap();

        let keys = storage.list_unified_keys().unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key_value, "sk-unikey-testkey000000000000001");

        // 5. Test router resolution
        let router = Router::new();
        let resolved = router.resolve(&storage, "sk-unikey-testkey000000000000001").unwrap();
        assert_eq!(resolved.configs.len(), 1);
        assert_eq!(resolved.api_keys.len(), 1);

        let request = ChatRequest {
            model: Some("gpt-4o".into()),
            messages: vec![],
            stream: false,
            temperature: 1.0,
            top_p: 1.0,
            max_tokens: 4096,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
        };

        let resolution = router.decide(&resolved, &request).unwrap();
        assert_eq!(resolution.provider, "openai");
        assert_eq!(resolution.model, "gpt-4o");
        assert_eq!(resolution.api_key, "sk-test-api-key");
        assert!((resolution.temperature - 0.7).abs() < 0.01);

        // 6. Verify encryption roundtrip
        let decrypted = storage.decrypt(&pk.encrypted_key).unwrap();
        assert_eq!(decrypted, "sk-test-api-key");
    }

    #[test]
    fn test_route_keyword_matching() {
        let storage = test_storage();

        // Setup: two configs, keyword-routed scene
        let pk = ProviderKey {
            id: "pk-x".into(),
            name: "Key".into(),
            provider: "deepseek".into(),
            encrypted_key: storage.encrypt("sk-code-key").unwrap(),
            base_url: None,
            created_at: 1,
        };
        storage.save_provider_key(&pk).unwrap();

        let config = ModelConfig {
            id: "cfg-code".into(),
            provider_key_id: "pk-x".into(),
            name: "Code Config".into(),
            model: "deepseek-chat".into(),
            temperature: 0.3,
            top_p: 1.0,
            max_tokens: 8192,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            system_prompt: None,
            extra_params: None,
            created_at: 1,
        };
        storage.save_model_config(&config).unwrap();

        let scene = Scene {
            id: "scene-kw".into(),
            name: "Keyword Scene".into(),
            description: "".into(),
            rules: vec![RouteRule {
                id: "rule-kw".into(),
                condition: RouteCondition::Keyword {
                    keywords: vec!["写代码".into(), "编程".into()],
                },
                model_config_id: "cfg-code".into(),
                priority: 1,
            }],
            created_at: 1,
            updated_at: 1,
        };
        storage.save_scene(&scene).unwrap();

        let uk = UnifiedKey {
            id: "uk-kw".into(),
            key_value: "sk-unikey-kwtest0000000000000001".into(),
            scene_id: "scene-kw".into(),
            name: "Test".into(),
            created_at: 1,
            last_used_at: None,
            usage_count: 0,
        };
        storage.save_unified_key(&uk).unwrap();

        let router = Router::new();
        let resolved = router.resolve(&storage, "sk-unikey-kwtest0000000000000001").unwrap();

        // Request about code → should match
        let code_req = ChatRequest {
            model: None,
            messages: vec![crate::providers::Message {
                role: "user".into(),
                content: crate::providers::MessageContent::Text("帮我写代码实现一个排序算法".into()),
            }],
            stream: false, temperature: 1.0, top_p: 1.0, max_tokens: 4096,
            frequency_penalty: 0.0, presence_penalty: 0.0,
        };
        let result = router.decide(&resolved, &code_req).unwrap();
        assert_eq!(result.model, "deepseek-chat");
    }
}
