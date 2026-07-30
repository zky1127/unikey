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
            // 确定数据目录
            let app_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            std::fs::create_dir_all(&app_dir).ok();

            let db_path = app_dir.join("unikey.db");

            // 初始化存储（用机器 ID 派生加密密钥）
            let machine_id = app.config().identifier.clone();
            let storage = Arc::new(Storage::new(db_path, &machine_id)?);

            // 初始化 Provider 注册表
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
