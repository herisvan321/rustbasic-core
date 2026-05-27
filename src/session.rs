use crate::Config;
use crate::session_manager::RustBasicSessionStore;
use sqlx::AnyPool;
use std::sync::Arc;
use std::sync::Mutex;
use serde_json::Value;

#[derive(Clone)]
pub struct Session {
    pub(crate) id: String,
    pub(crate) data: Arc<Mutex<serde_json::Map<String, Value>>>,
}

impl Session {
    pub fn new(id: String) -> Self {
        Self {
            id,
            data: Arc::new(Mutex::new(serde_json::Map::new())),
        }
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let guard = self.data.lock().unwrap();
        let val = guard.get(key)?;
        serde_json::from_value(val.clone()).ok()
    }

    pub fn set<T: serde::Serialize>(&self, key: &str, value: T) {
        if let Ok(val) = serde_json::to_value(value) {
            self.data.lock().unwrap().insert(key.to_string(), val);
        }
    }

    pub fn remove(&self, key: &str) -> Option<Value> {
        self.data.lock().unwrap().remove(key)
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

pub async fn setup_session(cfg: &Config) -> RustBasicSessionStore {
    let session_db_url = if cfg.session_driver == "file" {
        "sqlite:database/sessions.sqlite?mode=rwc".to_string()
    } else if cfg.db_connection == "mysql" {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            cfg.db_username, cfg.db_password, cfg.db_host, cfg.db_port, cfg.db_database
        )
    } else {
        format!("sqlite:database/{}.sqlite?mode=rwc", cfg.db_database)
    };

    sqlx::any::install_default_drivers();
    let session_pool = match AnyPool::connect(&session_db_url).await {
        Ok(pool) => pool,
        Err(e) => {
            let err_msg = e.to_string();
            if (err_msg.contains("1049") || err_msg.contains("Unknown database")) && cfg.db_connection == "mysql" {
                let root_url = format!("mysql://{}:{}@{}:{}", cfg.db_username, cfg.db_password, cfg.db_host, cfg.db_port);
                if let Ok(root_pool) = sqlx::MySqlPool::connect(&root_url).await {
                    let _ = sqlx::query(&format!("CREATE DATABASE IF NOT EXISTS `{}`", cfg.db_database)).execute(&root_pool).await;
                    AnyPool::connect(&session_db_url).await.expect("Gagal terhubung setelah membuat DB session")
                } else {
                    panic!("Gagal membuat database session otomatis: {:?}", e);
                }
            } else {
                panic!("Gagal terhubung ke database session: {:?}", e);
            }
        }
    };
    
    RustBasicSessionStore::new(session_pool)
}

pub async fn init_sessions(cfg: &Config) {
    let db_url = if cfg.session_driver == "file" {
        "sqlite:database/sessions.sqlite?mode=rwc".to_string()
    } else if cfg.db_connection == "mysql" {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            cfg.db_username, cfg.db_password, cfg.db_host, cfg.db_port, cfg.db_database
        )
    } else {
        format!("sqlite:database/{}.sqlite?mode=rwc", cfg.db_database)
    };

    sqlx::any::install_default_drivers();
    let pool = AnyPool::connect(&db_url).await.expect("Gagal terhubung ke database session");

    let sql = "CREATE TABLE IF NOT EXISTS sessions (
        id VARCHAR(255) PRIMARY KEY,
        payload TEXT NOT NULL,
        last_activity BIGINT NOT NULL,
        ip_address VARCHAR(45)
    )";

    sqlx::query(sql).execute(&pool).await.expect("Gagal membuat tabel session otomatis");
}
