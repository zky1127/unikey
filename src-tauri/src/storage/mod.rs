pub mod models;

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::Rng;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Mutex;

use models::*;

pub struct Storage {
    conn: Mutex<Connection>,
    encryption: Aes256Gcm,
}

impl Storage {
    /// 创建或打开数据库，使用密码派生加密密钥
    pub fn new(db_path: PathBuf, password: &str) -> Result<Self, String> {
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

        // 派生 AES-256 密钥
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let key = hasher.finalize();
        let key_bytes: [u8; 32] = key.into();
        let encryption = Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| e.to_string())?;

        let storage = Self {
            conn: Mutex::new(conn),
            encryption,
        };

        storage.init_tables()?;
        Ok(storage)
    }

    fn init_tables(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS provider_keys (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                provider TEXT NOT NULL,
                encrypted_key TEXT NOT NULL,
                base_url TEXT,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS model_configs (
                id TEXT PRIMARY KEY,
                provider_key_id TEXT NOT NULL,
                name TEXT NOT NULL,
                model TEXT NOT NULL,
                temperature REAL NOT NULL DEFAULT 1.0,
                top_p REAL NOT NULL DEFAULT 1.0,
                max_tokens INTEGER NOT NULL DEFAULT 4096,
                frequency_penalty REAL NOT NULL DEFAULT 0.0,
                presence_penalty REAL NOT NULL DEFAULT 0.0,
                system_prompt TEXT,
                extra_params TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (provider_key_id) REFERENCES provider_keys(id)
            );
            CREATE TABLE IF NOT EXISTS scenes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                rules TEXT NOT NULL DEFAULT '[]',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS unified_keys (
                id TEXT PRIMARY KEY,
                key_value TEXT NOT NULL UNIQUE,
                scene_id TEXT NOT NULL,
                name TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_used_at INTEGER,
                usage_count INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (scene_id) REFERENCES scenes(id)
            );
            CREATE TABLE IF NOT EXISTS proxy_logs (
                id TEXT PRIMARY KEY,
                unified_key_id TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER,
                output_tokens INTEGER,
                latency_ms INTEGER NOT NULL,
                success INTEGER NOT NULL,
                error TEXT
            );
            ",
        )
        .map_err(|e| e.to_string())
    }

    // ============ ProviderKey CRUD ============

    pub fn save_provider_key(&self, key: &ProviderKey) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO provider_keys (id, name, provider, encrypted_key, base_url, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![key.id, key.name, key.provider, key.encrypted_key, key.base_url, key.created_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_provider_keys(&self) -> Result<Vec<ProviderKey>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, name, provider, encrypted_key, base_url, created_at FROM provider_keys ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ProviderKey {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    provider: row.get(2)?,
                    encrypted_key: row.get(3)?,
                    base_url: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn delete_provider_key(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM provider_keys WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| e.to_string())?;
        // 级联删除关联的配置
        conn.execute(
            "DELETE FROM model_configs WHERE provider_key_id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ============ ModelConfig CRUD ============

    pub fn save_model_config(&self, config: &ModelConfig) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO model_configs (id, provider_key_id, name, model, temperature, top_p, max_tokens, frequency_penalty, presence_penalty, system_prompt, extra_params, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                config.id, config.provider_key_id, config.name, config.model,
                config.temperature, config.top_p, config.max_tokens,
                config.frequency_penalty, config.presence_penalty,
                config.system_prompt,
                config.extra_params.as_ref().map(|v| v.to_string()),
                config.created_at
            ],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_model_configs(&self) -> Result<Vec<ModelConfig>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, provider_key_id, name, model, temperature, top_p, max_tokens, frequency_penalty, presence_penalty, system_prompt, extra_params, created_at FROM model_configs ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let extra: Option<String> = row.get(10)?;
                Ok(ModelConfig {
                    id: row.get(0)?,
                    provider_key_id: row.get(1)?,
                    name: row.get(2)?,
                    model: row.get(3)?,
                    temperature: row.get(4)?,
                    top_p: row.get(5)?,
                    max_tokens: row.get(6)?,
                    frequency_penalty: row.get(7)?,
                    presence_penalty: row.get(8)?,
                    system_prompt: row.get(9)?,
                    extra_params: extra
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    created_at: row.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn delete_model_config(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM model_configs WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ============ Scene CRUD ============

    pub fn save_scene(&self, scene: &Scene) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let rules_json = serde_json::to_string(&scene.rules).unwrap_or_default();
        conn.execute(
            "INSERT OR REPLACE INTO scenes (id, name, description, rules, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![scene.id, scene.name, scene.description, rules_json, scene.created_at, scene.updated_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_scenes(&self) -> Result<Vec<Scene>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, name, description, rules, created_at, updated_at FROM scenes ORDER BY updated_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let rules_str: String = row.get(3)?;
                Ok(Scene {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    rules: serde_json::from_str(&rules_str).unwrap_or_default(),
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn delete_scene(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM scenes WHERE id = ?1", rusqlite::params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ============ UnifiedKey CRUD ============

    pub fn save_unified_key(&self, key: &UnifiedKey) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO unified_keys (id, key_value, scene_id, name, created_at, last_used_at, usage_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![key.id, key.key_value, key.scene_id, key.name, key.created_at, key.last_used_at, key.usage_count],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_unified_key(&self, key_value: &str) -> Result<Option<UnifiedKey>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, key_value, scene_id, name, created_at, last_used_at, usage_count FROM unified_keys WHERE key_value = ?1")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(rusqlite::params![key_value], |row| {
                Ok(UnifiedKey {
                    id: row.get(0)?,
                    key_value: row.get(1)?,
                    scene_id: row.get(2)?,
                    name: row.get(3)?,
                    created_at: row.get(4)?,
                    last_used_at: row.get(5)?,
                    usage_count: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        match rows.next() {
            Some(row) => Ok(Some(row.map_err(|e| e.to_string())?)),
            None => Ok(None),
        }
    }

    pub fn list_unified_keys(&self) -> Result<Vec<UnifiedKey>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, key_value, scene_id, name, created_at, last_used_at, usage_count FROM unified_keys ORDER BY created_at DESC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(UnifiedKey {
                    id: row.get(0)?,
                    key_value: row.get(1)?,
                    scene_id: row.get(2)?,
                    name: row.get(3)?,
                    created_at: row.get(4)?,
                    last_used_at: row.get(5)?,
                    usage_count: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn delete_unified_key(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "DELETE FROM unified_keys WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 更新统一 Key 使用记录
    pub fn record_usage(&self, key_value: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        conn.execute(
            "UPDATE unified_keys SET last_used_at = ?1, usage_count = usage_count + 1 WHERE key_value = ?2",
            rusqlite::params![now, key_value],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ============ Proxy Log ============

    pub fn save_proxy_log(&self, log: &ProxyLog) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO proxy_logs (id, unified_key_id, timestamp, provider, model, input_tokens, output_tokens, latency_ms, success, error) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                log.id, log.unified_key_id, log.timestamp,
                log.provider, log.model,
                log.input_tokens, log.output_tokens,
                log.latency_ms, log.success as i32, log.error,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ============ Encryption ============

    /// 加密 API Key
    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let mut rng = rand::thread_rng();
        let nonce_bytes: [u8; 12] = rng.gen();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = self
            .encryption
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| e.to_string())?;
        // nonce(12) + ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        Ok(BASE64.encode(&result))
    }

    /// 解密 API Key
    pub fn decrypt(&self, encrypted: &str) -> Result<String, String> {
        let data = BASE64.decode(encrypted).map_err(|e| e.to_string())?;
        if data.len() < 12 {
            return Err("Invalid encrypted data".to_string());
        }
        let nonce = Nonce::from_slice(&data[..12]);
        let plaintext = self
            .encryption
            .decrypt(nonce, &data[12..])
            .map_err(|e| e.to_string())?;
        String::from_utf8(plaintext).map_err(|e| e.to_string())
    }
}
