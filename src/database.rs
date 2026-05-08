use crate::Config;
use sea_orm::{Database, DatabaseConnection, ConnectOptions};
use std::time::Duration;
use colored::Colorize;

pub async fn connect(cfg: &Config) -> DatabaseConnection {
    // 1. Susun URL Koneksi berdasarkan pilihan di .env
    let db_url = if cfg.db_connection == "mysql" {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            cfg.db_username, cfg.db_password, cfg.db_host, cfg.db_port, cfg.db_database
        )
    } else {
        // Default ke SQLite
        format!("sqlite:database/{}.sqlite?mode=rwc", cfg.db_database)
    };

    // 2. Konfigurasi Opsi Koneksi
    let mut opt = ConnectOptions::new(db_url);
    opt.max_connections(20)
       .min_connections(5)
       .connect_timeout(Duration::from_secs(8))
       .idle_timeout(Duration::from_secs(8))
       .max_lifetime(Duration::from_secs(8))
       .sqlx_logging(true);

    // 3. Hubungkan ke Database dengan deteksi otomatis pembuatan DB (khusus MySQL)
    match Database::connect(opt.clone()).await {
        Ok(conn) => conn,
        Err(e) => {
            let err_msg = e.to_string();
            // Jika error 1049 (Unknown Database) dan ini MySQL, coba buat database
            if (err_msg.contains("1049") || err_msg.contains("Unknown database")) && cfg.db_connection == "mysql" {
                println!("{}", "⚠️  Database tidak ditemukan. Mencoba membuat database baru...".yellow());
                
                let root_url = format!(
                    "mysql://{}:{}@{}:{}",
                    cfg.db_username, cfg.db_password, cfg.db_host, cfg.db_port
                );
                
                if let Ok(pool) = sqlx::MySqlPool::connect(&root_url).await {
                    let create_query = format!("CREATE DATABASE IF NOT EXISTS `{}`", cfg.db_database);
                    if sqlx::query(&create_query).execute(&pool).await.is_ok() {
                        println!("✅ Database '{}' berhasil dibuat.", cfg.db_database.green());
                        return Database::connect(opt).await.expect("Gagal terhubung setelah membuat database");
                    }
                }
            }
            panic!("Gagal terhubung ke database: {:?}", e);
        }
    }
}
