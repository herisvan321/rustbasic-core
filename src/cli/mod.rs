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
    F: Fn() -> Pin<Box<dyn Future<Output = ()>>>,
    G: Fn() -> Pin<Box<dyn Future<Output = ()>>>
{
    dotenv().expect("❌ Error: File .env tidak ditemukan! Silakan salin .env.example menjadi .env sebelum menggunakan CLI.");
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    let command = &args[1];

    match command.as_str() {
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
        "migrate" => {
            migrate_fn().await;
        }
        "migrate:refresh" => {
            // Kita bisa menambah hook khusus untuk refresh jika perlu
            // Untuk sekarang, kita panggil fungsi database framework saja jika ia bisa handle secara generik
            // Tapi Migrator ada di App, jadi panggil migrate_fn saja atau tambah hook
            println!("{}", "⚠️  Fitur migrate:refresh menggunakan hook aplikasi.".dimmed());
            migrate_fn().await; 
        }
        "migrate:back" | "migrate:rollback" => {
            println!("{}", "⚠️  Fitur rollback menggunakan hook aplikasi.".dimmed());
            migrate_fn().await;
        }
        "route:list" => {
            monitoring::list_routes();
        }
        "build" => {
            builder::build_project();
        }
        "cache:clear" => {
            database::clear_cache().await;
        }
        "check:update" => {
            monitoring::check_updates();
        }
        "check:security" => {
            monitoring::check_security();
        }
        "key:generate" => {
            database::generate_app_key();
        }
        "make:auth" | "auth" => {
            if args.len() >= 3 && args[2] == "back" {
                auth::remove_auth().await;
            } else {
                auth::make_auth().await;
            }
        }
        "db:seed" => {
            seed_fn().await;
        }
        "make:seeder" => {
            if args.len() < 3 {
                println!("{}", "❌ Error: Nama seeder tidak ditentukan.".red().bold());
                return;
            }
            scaffolding::make_seeder(&args[2]);
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
    println!("  {} {} <Nama> [-m]   {}", "cargo rustbasic".blue(), "make:model".green(), "Membuat model Sea-ORM (dan migration Rust)".dimmed());
    println!("  {} {} <Nama>    {}", "cargo rustbasic".blue(), "make:migration".green(), "Membuat file migration Rust".dimmed());
    println!("  {} {} <Nama>  {}", "cargo rustbasic".blue(), "make:controller".green(), "Membuat controller Axum".dimmed());
    println!("  {} {} <Nama>  {}", "cargo rustbasic".blue(), "make:middleware".green(), "Membuat middleware Axum".dimmed());
    println!("  {} {}                  {}", "cargo rustbasic".blue(), "migrate".green(), "Menjalankan migrasi database (Sea-ORM)".dimmed());
    println!("  {} {}          {}", "cargo rustbasic".blue(), "migrate:refresh".green(), "Rollback semua dan jalankan kembali migrasi".dimmed());
    println!("  {} {}             {}", "cargo rustbasic".blue(), "migrate:back".green(), "Membatalkan migrasi terakhir (Rollback)".dimmed());
    println!("  {} {}               {}", "cargo rustbasic".blue(), "route:list".green(), "Menampilkan daftar route".dimmed());
    println!("  {} {}                    {}", "cargo rustbasic".blue(), "build".green(), "Membangun project dengan pilihan".dimmed());
    println!("  {} {}             {}", "cargo rustbasic".blue(), "check:update".green(), "Cek versi terbaru paket (dependencies)".dimmed());
    println!("  {} {}           {}", "cargo rustbasic".blue(), "check:security".green(), "Audit keamanan aplikasi".dimmed());
    println!("  {} {}               {}", "cargo rustbasic".blue(), "cache:clear".green(), "Membersihkan logs dan database sessions".dimmed());
    println!("  {} {}             {}", "cargo rustbasic".blue(), "key:generate".green(), "Membuat APP_KEY baru di file .env".dimmed());
    println!("  {} {}                   {}", "cargo rustbasic".blue(), "make:auth".green(), "Scaffold autentikasi (Login/Register)".dimmed());
    println!("  {} {}                   {}", "cargo rustbasic".blue(), "auth:back".red(), "Menghapus semua scaffolding autentikasi".dimmed());
    println!("  {} {}                  {}", "cargo rustbasic".blue(), "db:seed".green(), "Menjalankan seeder database".dimmed());
    println!("  {} {} <Nama>    {}", "cargo rustbasic".blue(), "make:seeder".green(), "Membuat file seeder baru".dimmed());
    println!("  {} {}                    {}", "cargo rustbasic".blue(), "serve".green(), "Menjalankan server dengan Auto-Reload".dimmed());
    println!("  {}                       {}", "cargo serve".blue(), "(Shortcut) Lebih cepat untuk menjalankan server".dimmed());

    println!();
}
