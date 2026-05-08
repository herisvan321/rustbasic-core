use std::process::Command;
use std::io::{self, Write};
use colored::*;

pub fn build_project() {
    println!("\n{}", "🚀 RustBasic Build Manager".magenta().bold());
    println!("{}", "--------------------------".magenta());
    
    // 1. Pilih Target
    println!("{}", "--- Pilih Target OS ---".cyan().bold());
    println!("1) Native (Sesuai OS Anda)");
    println!("2) Windows (x86_64-pc-windows-msvc)");
    println!("3) Linux (x86_64-unknown-linux-gnu)");
    println!("4) macOS ARM (aarch64-apple-darwin)");
    println!("5) Batal");
    print!("\n{}", "Masukkan pilihan target (1-5): ".bold());
    io::stdout().flush().unwrap();

    let mut target_choice = String::new();
    io::stdin().read_line(&mut target_choice).ok();
    let target_choice = target_choice.trim();

    if target_choice == "5" {
        println!("{}", "👋 Build dibatalkan.".yellow());
        return;
    }

    let target = match target_choice {
        "2" => Some("x86_64-pc-windows-msvc"),
        "3" => Some("x86_64-unknown-linux-gnu"),
        "4" => Some("aarch64-apple-darwin"),
        _ => None, // Native
    };

    // 2. Pilih Mode
    println!("\n{}", "--- Pilih Mode Build ---".cyan().bold());
    println!("1) Development");
    println!("2) Production (Release)");
    print!("\n{}", "Masukkan pilihan mode (1-2): ".bold());
    io::stdout().flush().unwrap();

    let mut mode_choice = String::new();
    io::stdin().read_line(&mut mode_choice).ok();
    let is_release = mode_choice.trim() == "2";

    // 3. Eksekusi Build
    println!("\n{}", "🛠️  Menyiapkan build...".blue());

    let has_zigbuild = Command::new("cargo")
        .arg("zigbuild")
        .arg("--version")
        .output()
        .is_ok();

    let mut cmd = if has_zigbuild && target.is_some() {
        println!("{}", "✨ Menggunakan cargo-zigbuild untuk kompilasi silang...".green().italic());
        let mut c = Command::new("cargo");
        c.arg("zigbuild");
        c
    } else {
        if let Some(t) = target {
            println!("{} {} {}", "📦 Menambahkan target".blue(), t.yellow(), "via rustup...".blue());
            Command::new("rustup")
                .args(["target", "add", t])
                .status()
                .ok();
        }
        let mut c = Command::new("cargo");
        c.arg("build");
        c
    };

    if is_release {
        cmd.arg("--release");
    }

    if let Some(t) = target {
        cmd.arg("--target").arg(t);
    }

    println!("{} {:?}", "🚀 Menjalankan:".blue().bold(), cmd);
    let status = cmd.status().expect("Gagal menjalankan perintah build");

    if status.success() {
        println!("\n{}", "✅ Build berhasil!".green().bold());
        if is_release {
            println!("{}", "📂 Output ada di folder target/release atau target/<target>/release".dimmed());
        }
    } else {
        println!("\n{}", "❌ Build gagal.".red().bold());
        println!("{}", "💡 Penyebab: Linker untuk target tersebut tidak ditemukan di sistem Anda.".yellow());
        
        if target_choice == "2" {
            println!("\n{}", "🔧 Cara memperbaiki untuk Windows:".cyan());
            println!("   Jalankan: {}", "brew install mingw-w64".white().on_black());
        } else if target_choice == "3" {
            println!("\n{}", "🔧 Cara memperbaiki untuk Linux:".cyan());
            println!("   Jalankan: {}", "brew install messense/macos-cross-toolchains/x86_64-unknown-linux-gnu".white().on_black());
        }
        
        println!("\n{}", "Atau gunakan 'cargo-zigbuild' untuk kompilasi silang yang lebih mudah:".cyan());
        println!("1. brew install zig");
        println!("2. cargo install cargo-zigbuild");
        println!("3. Gunakan '{}'", "cargo zigbuild --target <target>".white().on_black());
    }
}
