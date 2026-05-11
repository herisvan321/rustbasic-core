use std::env;
use dotenvy::dotenv;
use colored::*;
use std::future::Future;
use std::pin::Pin;

pub mod scaffolding;
pub mod database;
pub mod monitoring;
pub mod builder;
pub mod utils;
pub mod auth;

pub type AsyncHook = Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()>>>>;

pub async fn run_cli<F, G>(migrate_fn: F, seed_fn: G) 
where 
    F: Fn(String) -> Pin<Box<dyn Future<Output = Result<(), String>>>>,
    G: Fn() -> Pin<Box<dyn Future<Output = ()>>>
{
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    let command = &args[1];

    // Delegation: Jika kita di dalam project RustBasic dan menjalankan perintah 
    // yang butuh kompilasi lokal (seperti migrate), delegasikan ke 'cargo run'.
    // Ini memastikan migrasi lokal yang baru dibuat bisa terbaca.
    let commands_to_delegate = [
        "migrate", "migrate:refresh", "migrate:back", "migrate:rollback", 
        "db:seed", "route:list", "build"
    ];

    if env::var("RUSTBASIC_LOCAL").is_err() 
        && std::path::Path::new("Cargo.toml").exists() 
        && commands_to_delegate.contains(&command.as_str()) 
    {
        let status = std::process::Command::new("cargo")
            .args(["run", "-q", "--bin", "rustbasic-cli", "--"])
            .args(&args[1..])
            .env("RUSTBASIC_LOCAL", "true")
            .status();

        match status {
            Ok(s) => std::process::exit(s.code().unwrap_or(0)),
            Err(_) => {
                // Jika gagal (mungkin bukan project RustBasic yang valid), 
                // lanjutkan eksekusi menggunakan binary ini.
            }
        }
    }

    // .env hanya diwajibkan untuk perintah selain 'new'
    if command != "new" {
        let _ = dotenv(); // Coba muat .env jika ada
    }

    match command.as_str() {
        "-v" | "--version" | "version" => {
            println!("{} {}", "🛠️  RustBasic CLI Version:".magenta().bold(), env!("CARGO_PKG_VERSION").cyan().bold());
            return;
        }
        "serve" => {
            println!("\n   {} {}", "🚀".bold(), "Menjalankan server RustBasic dengan Auto-Reload...".magenta().bold());
            let status = std::process::Command::new("cargo")
                .args(["watch", "-c", "-q", "--no-ignore", "-i", "target", "-w", "src", "-w", ".env", "-w", "src/resources", "-x", "run"])
                .status()
                .expect("❌ Gagal menjalankan cargo watch. Pastikan cargo-watch sudah terinstall: cargo install cargo-watch");
            
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        },
        "make:model" => {
            if args.len() < 3 {
                println!("{}", "❌ Error: Nama model tidak ditentukan.".red().bold());
                return;
            }
            let model_name = &args[2];
            let with_migration = args.contains(&"-m".to_string());
            
            scaffolding::make_model(model_name);
            if with_migration {
                scaffolding::make_rust_migration(model_name);
            }
        }
        "make:migration" => {
            if args.len() < 3 {
                println!("{}", "❌ Error: Nama migration tidak ditentukan.".red().bold());
                return;
            }
            scaffolding::make_rust_migration(&args[2]);
        }
        "make:controller" => {
            if args.len() < 3 {
                println!("{}", "❌ Error: Nama controller tidak ditentukan.".red().bold());
                return;
            }
            scaffolding::make_controller(&args[2]);
        }
        "make:middleware" => {
            if args.len() < 3 {
                println!("{}", "❌ Error: Nama middleware tidak ditentukan.".red().bold());
                return;
            }
            scaffolding::make_middleware(&args[2]);
        }
        "migrate" | "migrate:refresh" | "migrate:back" | "migrate:rollback" => {
            if command == "migrate:refresh" {
                println!("{}", "🔄 Menyegarkan database (Refresh Migration)...".yellow());
            } else if command == "migrate:back" || command == "migrate:rollback" {
                println!("{}", "⏪ Membatalkan migrasi terakhir (Rollback 1 step)...".yellow());
            } else {
                println!("{}", "🚀 Menjalankan migrasi database...".cyan());
            }
            
            match migrate_fn(command.clone()).await {
                Ok(_) => {
                    println!("\n{} {}", "✅".green(), format!("Operasi '{}' berhasil diselesaikan.", command).green().bold());
                }
                Err(e) => {
                    eprintln!("\n{} {}", "❌ Error:".red().bold(), "Gagal menjalankan operasi database.".bold());
                    eprintln!("{} {}", "📝 Detail:".yellow(), e);
                    eprintln!("\n💡 {}", "Tips:".cyan().bold());
                    eprintln!("   Jika muncul error 'Migration file ... is missing', itu berarti database mencatat");
                    eprintln!("   migrasi yang sudah dijalankan, tapi file migrasinya sudah dihapus atau diubah.");
                    eprintln!("\n🛠️  {}", "Solusi:".cyan().bold());
                    eprintln!("   Hapus file database: 'rm database/rustbasic.sqlite' lalu jalankan migrasi lagi.");
                    std::process::exit(1);
                }
            }
        }
        "route:list" => {
            monitoring::list_routes();
        }
        "build" => {
            builder::build_project();
            println!("\n{} {}", "✅".green(), "Build project berhasil diselesaikan.".green().bold());
        }
        "cache:clear" => {
            database::clear_cache().await;
        }
        "check:update" => {
            monitoring::check_updates();
            println!("\n{} {}", "✅".green(), "Pemeriksaan update selesai.".green().bold());
        }
        "check:security" => {
            monitoring::check_security();
            println!("\n{} {}", "✅".green(), "Audit keamanan selesai.".green().bold());
        }
        "key:generate" => {
            database::generate_app_key();
        }
        "make:auth" | "auth" => {
            if args.len() >= 3 && args[2] == "back" {
                auth::remove_auth().await;
                println!("\n{} {}", "✅".green(), "Scaffolding autentikasi berhasil dihapus.".green().bold());
            } else {
                auth::make_auth().await;
                println!("\n{} {}", "✅".green(), "Scaffolding autentikasi berhasil dibuat.".green().bold());
            }
        }
        "db:seed" => {
            println!("{}", "🌱 Menjalankan seeder database...".cyan());
            seed_fn().await;
            println!("\n{} {}", "✅".green(), "Database seeding berhasil diselesaikan.".green().bold());
        }
        "make:seeder" => {
            if args.len() < 3 {
                println!("{}", "❌ Error: Nama seeder tidak ditentukan.".red().bold());
                return;
            }
            scaffolding::make_seeder(&args[2]);
        }
        "new" => {
            if args.len() < 3 {
                println!("{}", "❌ Error: Nama project tidak ditentukan.".red().bold());
                println!("Contoh: rustbasic new myapp");
                return;
            }
            let project_name = &args[2];

            // Cek apakah folder sudah ada
            if std::path::Path::new(project_name).exists() {
                println!("{} '{}' {}", "❌ Error: Folder".red().bold(), project_name.yellow(), "sudah ada! Silakan gunakan nama lain.".red().bold());
                return;
            }

            println!("\n✨ {} {}", "Membuat project baru:".bold(), project_name.cyan().bold());
            
            let status = std::process::Command::new("git")
                .args(["clone", "https://github.com/herisvan321/rustbasic", project_name])
                .status();

            match status {
                Ok(s) if s.success() => {
                    // Hapus folder .git agar menjadi project baru
                    let _ = std::process::Command::new("rm")
                        .args(["-rf", &format!("{}/.git", project_name)])
                        .status();

                    // Copy .env.example menjadi .env
                    let env_example = format!("{}/.env.example", project_name);
                    let env_file = format!("{}/.env", project_name);
                    if std::path::Path::new(&env_example).exists() {
                        match std::fs::copy(&env_example, &env_file) {
                            Ok(_) => println!("   {} .env.example → .env", "📋".blue()),
                            Err(e) => println!("   {} Gagal menyalin .env: {}", "⚠️".yellow(), e),
                        }
                    }

                    // Generate APP_KEY dan jalankan server
                    if std::path::Path::new(&env_file).exists() {
                        if std::env::set_current_dir(project_name).is_ok() {
                            database::generate_app_key();

                            // 3. Download Dependencies (cargo fetch)
                            println!("📦 {} {}", "Mengunduh dependencies...".bold(), "(Ini hanya dilakukan sekali saat inisialisasi)".dimmed());
                            let _ = std::process::Command::new("cargo")
                                .args(["fetch"])
                                .status();
                            
                            println!("\n✅ {} {}", "Project berhasil dibuat!".green().bold(), "Menyiapkan server...".dimmed());
                            
                            // Ambil APP_URL dari .env untuk dibuka di browser
                            let app_url = std::fs::read_to_string(".env")
                                .unwrap_or_default()
                                .lines()
                                .find(|line| line.starts_with("APP_URL="))
                                .map(|line| line.replace("APP_URL=", ""))
                                .unwrap_or_else(|| "http://localhost:4000".to_string());

                            // Open browser
                            utils::open_browser(&app_url);

                            // Jalankan serve (sama seperti perintah 'serve')
                            println!("\n   {} {}", "🚀".bold(), "Menjalankan server RustBasic dengan Auto-Reload...".magenta().bold());
                            let status = std::process::Command::new("cargo")
                                .args(["watch", "-c", "-q", "--no-ignore", "-i", "target", "-w", "src", "-w", ".env", "-w", "src/resources", "-x", "run"])
                                .status()
                                .expect("❌ Gagal menjalankan cargo watch. Pastikan cargo-watch sudah terinstall: cargo install cargo-watch");
                            
                            if !status.success() {
                                std::process::exit(status.code().unwrap_or(1));
                            }
                        }
                    }
                }
                _ => {
                    println!("{}", "❌ Gagal mengkloning starter template. Pastikan Anda memiliki koneksi internet dan git terinstall.".red());
                }
            }
        }
        "auth:back" => {
            auth::remove_auth().await;
        }
       
        _ => {
            println!("{} {}", "❌ Error: Perintah tidak dikenal:".red().bold(), command.yellow());
            print_help();
        }
    }
}

fn print_help() {
    println!("\n{}", "🛠️  RustBasic CLI".magenta().bold());
    println!("{}", "=================".magenta());
    println!("{}", "Penggunaan:".bold());
    println!("  {} {} <Nama>         {}", "rustbasic".blue(), "new".green(), "Membuat project RustBasic baru dari template".dimmed());
    println!("  {} {} <Nama> [-m]   {}", "rustbasic".blue(), "make:model".green(), "Membuat model Sea-ORM (dan migration Rust)".dimmed());
    println!("  {} {} <Nama>    {}", "rustbasic".blue(), "make:migration".green(), "Membuat file migration Rust".dimmed());
    println!("  {} {} <Nama>  {}", "rustbasic".blue(), "make:controller".green(), "Membuat controller Axum".dimmed());
    println!("  {} {} <Nama>  {}", "rustbasic".blue(), "make:middleware".green(), "Membuat middleware Axum".dimmed());
    println!("  {} {}                  {}", "rustbasic".blue(), "migrate".green(), "Menjalankan migrasi database (Sea-ORM)".dimmed());
    println!("  {} {}          {}", "rustbasic".blue(), "migrate:refresh".green(), "Rollback semua dan jalankan kembali migrasi".dimmed());
    println!("  {} {}             {}", "rustbasic".blue(), "migrate:back".green(), "Membatalkan migrasi terakhir (Rollback)".dimmed());
    println!("  {} {}               {}", "rustbasic".blue(), "route:list".green(), "Menampilkan daftar route".dimmed());
    println!("  {} {}                    {}", "rustbasic".blue(), "build".green(), "Membangun project dengan pilihan".dimmed());
    println!("  {} {}             {}", "rustbasic".blue(), "check:update".green(), "Cek versi terbaru paket (dependencies)".dimmed());
    println!("  {} {}           {}", "rustbasic".blue(), "check:security".green(), "Audit keamanan aplikasi".dimmed());
    println!("  {} {}               {}", "rustbasic".blue(), "cache:clear".green(), "Membersihkan logs dan database sessions".dimmed());
    println!("  {} {}             {}", "rustbasic".blue(), "key:generate".green(), "Membuat APP_KEY baru di file .env".dimmed());
    println!("  {} {}                   {}", "rustbasic".blue(), "make:auth".green(), "Scaffold autentikasi (Login/Register)".dimmed());
    println!("  {} {}                   {}", "rustbasic".blue(), "auth:back".red(), "Menghapus semua scaffolding autentikasi".dimmed());
    println!("  {} {}                  {}", "rustbasic".blue(), "db:seed".green(), "Menjalankan seeder database".dimmed());
    println!("  {} {} <Nama>    {}", "rustbasic".blue(), "make:seeder".green(), "Membuat file seeder baru".dimmed());
    println!("  {} {}                   {}", "rustbasic".blue(), "serve".green(), "Menjalankan server dengan Auto-Reload".dimmed());
    println!("  {} {}                 {}", "rustbasic".blue(), "version".green(), "Menampilkan versi CLI saat ini".dimmed());
    println!("  {} {}                      {}", "rustbasic".blue(), "-v".green(), "Shortcut untuk menampilkan versi".dimmed());
    println!("  {} {}                  {}", "rustbasic".blue(), "cargo serve".green(), "(Shortcut) Lebih cepat untuk menjalankan server".dimmed());

    println!();
}
