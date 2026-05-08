use std::fs;
use crate::Config;
use crate::database::connect;
use base64::{Engine as _, engine::general_purpose};
use rand::Rng;
use regex::Regex;
use colored::*;
use sea_orm::ConnectionTrait;

pub async fn clear_cache() {
    println!("\n{}", "🧹 Cleaning Cache & Logs...".magenta().bold());

    // 1. Clear Logs
    let log_dir = "storage/logs";
    if let Ok(entries) = fs::read_dir(log_dir) {
        let mut count = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let _ = fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&path);
                count += 1;
            }
        }
        println!("   {} Folder storage/logs telah dikosongkan. ({} file dibersihkan)", "✅ Logs:".green(), count);
    } else {
        println!("   {} Folder storage/logs tidak ditemukan.", "⚠️  Logs:".yellow());
    }

    // 2. Clear Sessions in DB
    let cfg = Config::load();
    let db = connect(&cfg).await;
    
    let truncate_sql = if cfg.db_connection == "mysql" {
        "TRUNCATE TABLE sessions"
    } else {
        "DELETE FROM sessions"
    };

    match db.execute(sea_orm::Statement::from_string(cfg.db_backend(), truncate_sql.to_string())).await {
        Ok(_) => println!("   {} Tabel sessions telah dikosongkan.", "✅ Sessions:".green()),
        Err(e) => println!("   {} Gagal membersihkan tabel sessions. ({})", "❌ Error:".red(), e),
    }

    println!("\n{}", "✨ Cache berhasil dibersihkan!".green().bold());
}

pub fn generate_app_key() {
    println!("\n{}", "🔑 Generating Application Key...".magenta().bold());

    let mut key = [0u8; 32];
    rand::rng().fill_bytes(&mut key);
    
    let encoded = general_purpose::STANDARD.encode(key);
    let key_str = format!("base64:{}", encoded);
    
    let env_path = ".env";
    match fs::read_to_string(env_path) {
        Ok(content) => {
            let re = Regex::new(r"(?m)^APP_KEY=.*").unwrap();
            let new_content = if re.is_match(&content) {
                re.replace(&content, &format!("APP_KEY={}", key_str)).to_string()
            } else {
                format!("{}\nAPP_KEY={}", content.trim_end(), key_str)
            };

            if let Err(e) = fs::write(env_path, new_content) {
                println!("{} Gagal menulis ke file .env: {}", "❌ Error:".red(), e);
            } else {
                println!("{} {}", "✅ Application key set successfully:".green(), key_str.cyan());
                println!("{}", "💡 Pastikan untuk tidak membagikan APP_KEY ini ke publik!".dimmed());
            }
        }
        Err(_) => {
            println!("{} File .env tidak ditemukan.", "❌ Error:".red());
        }
    }
}
