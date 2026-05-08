use axum_session::{SessionConfig, SessionStore, Key};
use crate::Config;
use crate::session_manager::RustBasicSessionStore;
use sha2::{Sha512, Digest};
use sqlx::AnyPool;
use sea_orm::{ConnectionTrait, Database, sea_query, Iden};
use sea_query::{Table, ColumnDef};

#[derive(Iden)]
enum Sessions {
    Table,
    Id,
    Payload,
    LastActivity,
    IpAddress,
}

pub async fn setup_session(cfg: &Config) -> SessionStore<RustBasicSessionStore> {
    // 1. Decode APP_KEY
    let key_bytes = if cfg.app_key.starts_with("base64:") {
        use base64::{Engine as _, engine::general_purpose};
        general_purpose::STANDARD.decode(&cfg.app_key[7..]).unwrap_or_else(|_| cfg.app_key.as_bytes().to_vec())
    } else {
        cfg.app_key.as_bytes().to_vec()
    };
    
    // 2. Derive 64-byte key using Sha512
    let mut hasher = Sha512::new();
    hasher.update(&key_bytes);
    let final_key = hasher.finalize();
    let session_key = Key::from(&final_key);

    // 3. Setup Session Config
    let session_config = SessionConfig::default()
        .with_table_name("sessions")
        .with_key(session_key);

    // 4. Determine Session DB URL
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

    // 5. Connect and Create Store
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
    
    SessionStore::<RustBasicSessionStore>::new(
        Some(RustBasicSessionStore::new(session_pool)), 
        session_config
    ).await.expect("Gagal menginisialisasi SessionStore")
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

    let db = Database::connect(&db_url).await.expect("Gagal terhubung ke database session");
    let builder = db.get_database_backend();

    // 2. Auto-Create Table Sessions jika belum ada menggunakan Sea-ORM
    let table = Table::create()
        .table(Sessions::Table)
        .if_not_exists()
        .col(ColumnDef::new(Sessions::Id).string_len(255).primary_key())
        .col(ColumnDef::new(Sessions::Payload).text().not_null())
        .col(ColumnDef::new(Sessions::LastActivity).big_integer().not_null())
        .col(ColumnDef::new(Sessions::IpAddress).string_len(45))
        .to_owned();

    db.execute(builder.build(&table))
        .await
        .expect("Gagal membuat tabel session otomatis");
}
