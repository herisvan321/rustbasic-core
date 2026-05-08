# 🚀 RustBasic Core

**RustBasic Core** is the high-performance engine behind the RustBasic Framework. It provides a modular, agent-centric architecture for building modern web applications with Axum, Sea-ORM, and MiniJinja.

[![Crates.io](https://img.shields.io/crates/v/rustbasic-core.svg)](https://crates.io/crates/rustbasic-core)
[![Documentation](https://docs.rs/rustbasic-core/badge.svg)](https://docs.rs/rustbasic-core)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## ✨ Features

- ⚡ **Axum Powered**: Blazing fast HTTP server wrapper.
- 🗄️ **Sea-ORM Integration**: Robust async ORM with auto-migration support.
- 🎨 **MiniJinja Views**: Standard HTML templates with Jinja2-like syntax.
- 🛡️ **Security First**: Built-in CSRF protection, secure sessions, and rate limiting.
- 🛠️ **CLI Scaffolding**: Rapidly generate controllers, models, and migrations.
- 📧 **Mailer Service**: Integrated SMTP support via Lettre.

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rustbasic-core = "0.0.4"
```

## 🚀 Quick Start

```rust
use rustbasic_core::Config;

#[tokio::main]
async fn main() {
    // 1. Load Configuration
    let cfg = Config::load();

    // 2. Connect to Database
    let db = rustbasic_core::database::connect(&cfg).await;

    // 3. Build your Router
    let app_router = rustbasic_core::Router::new();

    // 4. Start Server
    rustbasic_core::server::start_server(cfg, session_store, static_files, db, app_router).await;
}
```

## 📖 Documentation

For full documentation and the starter template, visit the main [RustBasic Repository](https://github.com/herisvan321/rustbasic).

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

Built with ❤️ by the RustBasic Team.
