use std::fs;
use colored::*;
use regex::Regex;
use super::scaffolding::update_controller_mod_rs;

pub async fn make_auth() {
    println!("\n{}", "🔐 Scaffolding Authentication...".magenta().bold());

    // 1. Create src/routes/auth.rs
    let auth_route_path = "src/routes/auth.rs";
    let auth_route_template = r#"use rustbasic_core::axum::{Router, routing::{get, post}, middleware::from_fn};
use crate::app::http::controllers::auth;
use crate::app::http::middleware::auth::guest_middleware;
use rustbasic_core::server::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", get(auth::auth_controller::AuthController::login_page))
        .route("/login", post(auth::auth_controller::AuthController::login))
        .route("/register", get(auth::auth_controller::AuthController::register_page))
        .route("/register", post(auth::auth_controller::AuthController::register))
        .route("/forgot-password", get(auth::auth_controller::AuthController::forgot_password_page))
        .route("/forgot-password", post(auth::auth_controller::AuthController::send_reset_link))
        .route("/reset-password", get(auth::auth_controller::AuthController::reset_password_page))
        .route("/reset-password", post(auth::auth_controller::AuthController::update_password))
        .layer(from_fn(guest_middleware))
}
"#;
    if !std::path::Path::new(auth_route_path).exists() {
        fs::write(auth_route_path, auth_route_template).ok();
        println!("   {} {}", "✅ Created:".green(), auth_route_path.cyan());
    } else {
        println!("   {} {}", "⚠️  Exists:".yellow(), auth_route_path.cyan());
    }

    // 2. Update src/routes/mod.rs
    let routes_mod_path = "src/routes/mod.rs";
    if let Ok(mut content) = fs::read_to_string(routes_mod_path)
        && !content.contains("pub mod auth;") {
            content.push_str("pub mod auth;\n");
            fs::write(routes_mod_path, content).ok();
            println!("   {} {}", "📝 Updated:".blue(), routes_mod_path.cyan());
        }

    // 3. Update src/routes/web.rs
    let web_route_path = "src/routes/web.rs";
    if let Ok(mut content) = fs::read_to_string(web_route_path)
        && !content.contains("use crate::routes::auth as auth_routes;") {
            content = content.replace("use rustbasic_core::axum::{Router, routing::get};", "use rustbasic_core::axum::{Router, routing::{get, post}, middleware::from_fn};");
            content = content.replace("use rustbasic_core::server::AppState;", "use crate::app::http::controllers::{auth, dashboard_controller};\nuse crate::app::http::middleware::auth::auth_middleware;\nuse rustbasic_core::server::AppState;\nuse crate::routes::auth as auth_routes;");

            let merge_logic = r#"let auth_protected_routes = Router::new()
        .route("/dashboard", get(dashboard_controller::DashboardController::index))
        .route("/logout", post(auth::auth_controller::AuthController::logout))
        .layer(from_fn(auth_middleware));

    Router::new()
        .route("/", get(welcome_controller::index))
        .route("/dev", get(welcome_controller::dev_info))
        .merge(auth_routes::router())
        .merge(auth_protected_routes)"#;

            // Use regex for more robust replacement (includes leading spaces)
            let re = Regex::new(r#"(?s)Router::new\(\s*\n\s*\.route\("/", get\(welcome_controller::index\)\)\s*\n\s*\.route\("/dev", get\(welcome_controller::dev_info\)\)"#).unwrap();
            if re.is_match(&content) {
                content = re.replace(&content, merge_logic).to_string();
            } else {
                // Fallback for simple replacement
                content = content.replace("Router::new()\n        .route(\"/\", get(welcome_controller::index))\n        .route(\"/dev\", get(welcome_controller::dev_info))", merge_logic);
            }
            
            fs::write(web_route_path, content).ok();
            println!("   {} {}", "📝 Updated:".blue(), web_route_path.cyan());
        }

    // 3.1 Create Password Resets Migration
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let migration_name = format!("m{}_create_password_resets_table", timestamp);
    let migration_path = format!("database/migrations/{}.rs", migration_name);
    
    // Check if any password reset migration already exists
    let mut exists = false;
    if let Ok(entries) = std::fs::read_dir("database/migrations") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str()
                && name.ends_with("_create_password_resets_table.rs") {
                    exists = true;
                    println!("   {} {}", "⚠️  Exists:".yellow(), name.cyan());
                    break;
                }
        }
    }

    if !exists {
        let migration_template = r#"use sea_orm_migration::prelude::*;
use async_trait::async_trait;

#[derive(Iden)]
enum PasswordResets {
    Table,
    Email,
    Token,
    CreatedAt,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PasswordResets::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PasswordResets::Email).string().not_null().primary_key())
                    .col(ColumnDef::new(PasswordResets::Token).string().not_null())
                    .col(
                        ColumnDef::new(PasswordResets::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PasswordResets::Table).to_owned())
            .await
    }
}
"#.to_string();
        fs::write(&migration_path, migration_template).ok();
        
        super::scaffolding::update_migration_mod_rs(&migration_name);
        println!("   {} {}", "✅ Created:".green(), format!("Migration {}", migration_name).cyan());
    }

    // 4. Create Controller Folder & mod.rs
    let auth_controller_dir = "src/app/http/controllers/auth";
    fs::create_dir_all(auth_controller_dir).ok();
    let auth_controller_mod = "src/app/http/controllers/auth/mod.rs";
    if !std::path::Path::new(auth_controller_mod).exists() {
        fs::write(auth_controller_mod, "pub mod auth_controller;").ok();
    }
    update_controller_mod_rs("auth");

    // 4.1 Create Auth Middleware
    let auth_middleware_dir = "src/app/http/middleware";
    fs::create_dir_all(auth_middleware_dir).ok();
    let auth_middleware_path = "src/app/http/middleware/auth.rs";
    if !std::path::Path::new(auth_middleware_path).exists() {
        let middleware_template = r#"use rustbasic_core::axum::{
    middleware::Next,
    response::{IntoResponse, Redirect},
    extract::Request,
};
use rustbasic_core::responses::ResponseHelper;
use rustbasic_core::session_manager::RustBasicSessionStore;
use rustbasic_core::axum_session::Session;

pub async fn auth_middleware(req: Request, next: Next) -> impl IntoResponse {
    let session = req.extensions().get::<Session<RustBasicSessionStore>>().unwrap();
    if session.get::<i32>("user_id").is_none() {
        return ResponseHelper::redirect_with_error("/login", "Silakan login terlebih dahulu", session.clone()).into_response();
    }
    next.run(req).await
}

pub async fn guest_middleware(req: Request, next: Next) -> impl IntoResponse {
    let session = req.extensions().get::<Session<RustBasicSessionStore>>().unwrap();
    if session.get::<i32>("user_id").is_some() {
        return Redirect::to("/dashboard").into_response();
    }
    next.run(req).await
}
"#;
        fs::write(auth_middleware_path, middleware_template).ok();
        
        // Update src/app/http/middleware/mod.rs
        let middleware_mod_path = "src/app/http/middleware/mod.rs";
        if let Ok(mut content) = fs::read_to_string(middleware_mod_path)
            && !content.contains("pub mod auth;") {
                content.push_str("pub mod auth;\n");
                fs::write(middleware_mod_path, content).ok();
            }
        println!("   {} {}", "✅ Created:".green(), auth_middleware_path.cyan());
    }

    // 4.1 Create Password Resets Model
    let model_path = "src/app/models/password_resets.rs";
    if !std::path::Path::new(model_path).exists() {
        let model_template = r#"use rustbasic_core::sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "password_resets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub email: String,
    pub token: String,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
"#;
        fs::write(model_path, model_template).ok();
        
        // Update src/app/models/mod.rs
        let models_mod_path = "src/app/models/mod.rs";
        if let Ok(mut content) = fs::read_to_string(models_mod_path)
            && !content.contains("pub mod password_resets;") {
                content.push_str("pub mod password_resets;\n");
                fs::write(models_mod_path, content).ok();
            }
        println!("   {} {}", "✅ Created:".green(), "Model password_resets".cyan());
    }

    let auth_controller_path = "src/app/http/controllers/auth/auth_controller.rs";
    if !std::path::Path::new(auth_controller_path).exists() {
        let controller_template = r#"/* ---------------------------------------------------------
 * 📑 LABEL: AUTH CONTROLLER (auth/auth_controller.rs)
 * Menangani pendaftaran, login, dan logout user.
 * --------------------------------------------------------- */

use crate::app::view;
use crate::app::models::users;
use rustbasic_core::requests::Request;
use rustbasic_core::responses::ResponseHelper;
use rustbasic_core::server::AppState;
use rustbasic_core::axum::{response::IntoResponse, extract::State};
use rustbasic_core::bcrypt::{hash, verify, DEFAULT_COST};
use rustbasic_core::uuid::Uuid;
use serde::Deserialize;
use validator::Validate;
use rustbasic_core::mail::MailService;
use rustbasic_core::minijinja::context;
use rustbasic_core::sea_orm::{EntityTrait, ColumnTrait, QueryFilter, Set};

#[derive(Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, message = "Nama minimal 3 karakter"))]
    pub name: String,
    
    #[validate(email(message = "Format email tidak valid"))]
    pub email: String,
    
    #[validate(length(min = 8, message = "Password minimal 8 karakter"))]
    pub password: String,
}

#[derive(Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Format email tidak valid"))]
    pub email: String,
    pub password: String,
    pub remember: Option<String>,
}

#[derive(Deserialize, Validate)]
pub struct ForgotPasswordRequest {
    #[validate(email(message = "Format email tidak valid"))]
    pub email: String,
}

#[derive(Deserialize, Validate)]
pub struct ResetPasswordRequest {
    pub token: String,
    #[validate(length(min = 8, message = "Password minimal 8 karakter"))]
    pub password: String,
}

pub struct AuthController;

impl AuthController {
    /// Menampilkan halaman login
    pub async fn login_page(req: Request) -> impl IntoResponse {
        view(&req, "auth/login.rb.html", context! { title => "Login" })
    }

    /// Menampilkan halaman register
    pub async fn register_page(req: Request) -> impl IntoResponse {
        view(&req, "auth/register.rb.html", context! { title => "Daftar Akun" })
    }

    /// Proses Pendaftaran
    pub async fn register(State(state): State<AppState>, req: Request) -> impl IntoResponse {
        // 1. Validasi Input
        let data = match req.validate::<RegisterRequest>() {
            Ok(d) => d,
            Err(_) => return ResponseHelper::redirect("/register"),
        };

        // 2. Cek apakah email sudah terdaftar
        let existing = users::Entity::find()
            .filter(users::Column::Email.eq(&data.email))
            .one(&state.db)
            .await
            .ok()
            .flatten();

        if existing.is_some() {
            return ResponseHelper::redirect_with_error("/register", "Email sudah terdaftar", req.session);
        }

        // 3. Hash Password
        let hashed = hash(data.password, DEFAULT_COST).unwrap();

        // 4. Simpan ke Database
        let new_user = users::ActiveModel {
            name: Set(data.name),
            email: Set(data.email),
            password: Set(hashed),
            ..Default::default()
        };

        if let Err(e) = users::Entity::insert(new_user).exec(&state.db).await {
            rustbasic_core::tracing::error!("Gagal menyimpan user: {}", e);
            return ResponseHelper::redirect_with_error("/register", "Gagal mendaftar, coba lagi.", req.session);
        }

        ResponseHelper::redirect_with_success("/login", "Pendaftaran berhasil! Silakan login.", req.session)
    }

    /// Proses Login
    pub async fn login(State(state): State<AppState>, req: Request) -> impl IntoResponse {
        // 1. Validasi Input
        let data = match req.validate::<LoginRequest>() {
            Ok(d) => d,
            Err(_) => return ResponseHelper::redirect("/login"),
        };

        // 2. Ambil User dari DB
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(&data.email))
            .one(&state.db)
            .await
            .ok()
            .flatten();

        if let Some(u) = user {
            // 3. Verifikasi Password
            if verify(data.password, &u.password).unwrap_or(false) {
                // 4. Set Session
                req.session.set("user_id", u.id);
                
                // Handle "Remember Me"
                if data.remember.is_some() {
                    // Set session expiration to 30 days if remember is checked
                    // Note: implementation depends on axum_session configuration
                    rustbasic_core::tracing::info!("Remember me checked for user: {}", u.email);
                }

                return ResponseHelper::redirect_with_success("/dashboard", "Selamat datang kembali!", req.session);
            }
        }

        ResponseHelper::redirect_with_error("/login", "Email atau password salah", req.session)
    }

    /// Menampilkan halaman lupa password
    pub async fn forgot_password_page(req: Request) -> impl IntoResponse {
        view(&req, "auth/forgot.rb.html", context! { title => "Lupa Password" })
    }

    /// Kirim link reset password
    pub async fn send_reset_link(State(state): State<AppState>, req: Request) -> impl IntoResponse {
        let data = match req.validate::<ForgotPasswordRequest>() {
            Ok(d) => d,
            Err(_) => return ResponseHelper::redirect("/forgot-password"),
        };

        // 1. Cek apakah user ada
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(&data.email))
            .one(&state.db)
            .await
            .ok()
            .flatten();

        if let Some(u) = user {
            // 2. Generate Token
            let token = Uuid::new_v4().to_string();

            // 3. Simpan Token
            let reset = crate::app::models::password_resets::ActiveModel {
                email: Set(u.email.clone()),
                token: Set(token.clone()),
                created_at: Set(rustbasic_core::chrono::Utc::now().naive_utc()),
            };

            let _ = crate::app::models::password_resets::Entity::insert(reset)
                .on_conflict(
                    rustbasic_core::sea_orm::sea_query::OnConflict::column(crate::app::models::password_resets::Column::Email)
                        .update_column(crate::app::models::password_resets::Column::Token)
                        .update_column(crate::app::models::password_resets::Column::CreatedAt)
                        .to_owned()
                )
                .exec(&state.db)
                .await;

            // 4. Kirim Email (Gunakan Config::load().mail_*)
            let config = rustbasic_core::Config::load();
            let app_name = std::env::var("APP_NAME").unwrap_or_else(|_| "RustBasic".to_string());
            let reset_url = format!("{}/reset-password?token={}", config.app_url, token);

            let subject = format!("Reset Password - {}", app_name);
            let body = rustbasic_core::view::render_to_string("emails/reset.rb.html", context! {
                app_name => app_name,
                reset_url => reset_url,
            });

            if let Err(e) = MailService::send_email(&u.email, &subject, &body).await {
                rustbasic_core::tracing::error!("Gagal mengirim email reset: {}", e);
            }

            rustbasic_core::tracing::info!("Reset link for {}: {}", u.email, reset_url);
        }

        ResponseHelper::redirect_with_success("/login", "Jika email terdaftar, link reset password akan dikirim.", req.session)
    }

    /// Menampilkan halaman reset password
    pub async fn reset_password_page(req: Request) -> impl IntoResponse {
        let token = req.input_as_str("token").unwrap_or_default();
        view(&req, "auth/reset.rb.html", context! { title => "Reset Password", token => token })
    }

    /// Proses update password baru
    pub async fn update_password(State(state): State<AppState>, req: Request) -> impl IntoResponse {
        let data = match req.validate::<ResetPasswordRequest>() {
            Ok(d) => d,
            Err(_) => return ResponseHelper::redirect("/login"),
        };

        // 1. Cari Token
        let reset = crate::app::models::password_resets::Entity::find()
            .filter(crate::app::models::password_resets::Column::Token.eq(&data.token))
            .one(&state.db)
            .await
            .ok()
            .flatten();

        if let Some(r) = reset {
            // 2. Cek Kadaluarsa (60 Menit)
            let now = rustbasic_core::chrono::Utc::now().naive_utc();
            let duration = now.signed_duration_since(r.created_at);
            
            if duration.num_minutes() > 60 {
                // Hapus token yang sudah kadaluarsa
                let _ = crate::app::models::password_resets::Entity::delete_by_id(r.email.clone())
                    .exec(&state.db)
                    .await;
                    
                return ResponseHelper::redirect_with_error("/login", "Tautan reset password sudah kadaluarsa (melebihi 60 menit).", req.session);
            }

            // 3. Hash Password Baru
            let hashed = rustbasic_core::bcrypt::hash(data.password, rustbasic_core::bcrypt::DEFAULT_COST).unwrap();

            // 4. Update User
            let _ = users::Entity::update_many()
                .col_expr(users::Column::Password, rustbasic_core::sea_orm::sea_query::Expr::value(hashed))
                .filter(users::Column::Email.eq(&r.email))
                .exec(&state.db)
                .await;

            // 5. Hapus Token
            let _ = crate::app::models::password_resets::Entity::delete_by_id(r.email)
                .exec(&state.db)
                .await;

            return ResponseHelper::redirect_with_success("/login", "Password berhasil diubah. Silakan login.", req.session);
        }

        ResponseHelper::redirect_with_error("/login", "Token tidak valid atau sudah kadaluarsa.", req.session)
    }

    /// Proses Logout
    pub async fn logout(req: Request) -> impl IntoResponse {
        req.session.remove("user_id");
        ResponseHelper::redirect_with_success("/", "Anda telah keluar.", req.session)
    }
}
"#;
        fs::write(auth_controller_path, controller_template).ok();
        println!("   {} {}", "✅ Created:".green(), auth_controller_path.cyan());
    }

    // 5. Views
    let auth_view_dir = "src/resources/views/auth";
    fs::create_dir_all(auth_view_dir).ok();
    
    let login_template = r##"{% extends "layouts/app.rb.html" %}

{% block title %}Login - RustBasic{% endblock %}

{% block content %}
<div class="split-screen">
    <!-- Sisi Visual -->
    <div class="split-side-visual">
        <div class="visual-inner" style="max-width: 600px;">
            <div style="margin-bottom: 2rem;">
                <span class="badge" style="background: rgba(255,255,255,0.2); color: #fff; border: none;">RUSTBASIC FRAMEWORK</span>
            </div>
            <h1 style="font-size: 3.5rem; font-weight: 900; line-height: 1.1; margin-bottom: 1.5rem; text-shadow: 0 10px 20px rgba(0,0,0,0.1);">
                Selamat Datang <br> <span style="color: rgba(255,255,255,0.8);">Kembali</span>
            </h1>
            <p style="font-size: 1.2rem; opacity: 0.9; margin-bottom: 2.5rem; font-weight: 500;">
                Masuk untuk melanjutkan pengembangan aplikasi modern Anda dengan kecepatan dan keamanan Rust.
            </p>
            <div class="tech-stack" style="justify-content: center; margin-top: 1rem;">
                <span class="badge">Axum</span>
                <span class="badge">Sea-ORM</span>
                <span class="badge">Minijinja</span>
            </div>
        </div>
    </div>

    <!-- Sisi Form -->
    <div class="split-side-content">
        <div class="content-container">
            <div style="margin-bottom: 3rem;">
                <h2 class="title" style="font-size: 2.8rem; margin-bottom: 0.5rem;">Login</h2>
                <p class="text-muted" style="font-weight: 500;">Silakan masukkan akun Anda untuk melanjutkan.</p>
            </div>

            <form hx-post="/login" hx-target="body" hx-push-url="true" hx-indicator="#indicator" style="display: flex; flex-direction: column; gap: 1.5rem;">
                <div>
                    <label class="form-label">Email Address</label>
                    <input type="email" name="email" class="form-control" placeholder="nama@email.com" value="{{ old.email }}" required autofocus>
                    {% if errors.email %}
                        <div style="color: var(--secondary); font-size: 0.85rem; margin-top: 0.5rem; font-weight: 600;">{{ errors.email }}</div>
                    {% endif %}
                </div>

                <div>
                    <label class="form-label">Password</label>
                    <input type="password" name="password" class="form-control" placeholder="••••••••" required>
                    {% if errors.password %}
                        <div style="color: var(--secondary); font-size: 0.85rem; margin-top: 0.5rem; font-weight: 600;">{{ errors.password }}</div>
                    {% endif %}
                </div>

                <div style="display: flex; justify-content: space-between; align-items: center;">
                    <label style="display: flex; align-items: center; gap: 0.6rem; font-size: 0.9rem; cursor: pointer; color: var(--text-muted); font-weight: 500;">
                        <input type="checkbox" name="remember" value="1" style="width: 18px; height: 18px; accent-color: var(--primary);"> 
                        Ingat Saya
                    </label>
                    <a href="/forgot-password" style="font-size: 0.9rem; font-weight: 700; color: var(--primary); text-decoration: none;">Lupa Password?</a>
                </div>

                <div style="margin-top: 1rem;">
                    <button type="submit" class="btn btn-primary w-100" style="padding: 1.25rem;">
                        MASUK KE DASHBOARD
                    </button>
                </div>

                <p class="text-center" style="font-size: 0.95rem; color: var(--text-muted); margin-top: 1rem;">
                    Belum punya akun? <a href="/register" style="font-weight: 800; color: var(--accent); text-decoration: none;">Daftar Sekarang</a>
                </p>
            </form>
        </div>
    </div>
</div>
{% endblock %}
"##;

    let register_template = r##"{% extends "layouts/app.rb.html" %}

{% block title %}Daftar - RustBasic{% endblock %}

{% block content %}
<div class="split-screen">
    <!-- Sisi Visual -->
    <div class="split-side-visual" style="background: linear-gradient(135deg, var(--secondary), var(--accent), var(--primary));">
        <div class="visual-inner" style="max-width: 600px;">
            <div style="margin-bottom: 2rem;">
                <span class="badge" style="background: rgba(255,255,255,0.2); color: #fff; border: none;">JOIN REVOLUTION</span>
            </div>
            <h1 style="font-size: 3.5rem; font-weight: 900; line-height: 1.1; margin-bottom: 1.5rem; text-shadow: 0 10px 20px rgba(0,0,0,0.1);">
                Mulai Perjalanan <br> <span style="color: rgba(255,255,255,0.8);">Anda</span>
            </h1>
            <p style="font-size: 1.2rem; opacity: 0.9; margin-bottom: 2.5rem; font-weight: 500;">
                Bangun infrastruktur digital yang kokoh dengan framework yang mengutamakan keamanan dan performa maksimal.
            </p>
            <div style="display: flex; gap: 1rem; justify-content: center;">
                <div style="text-align: center;">
                    <div style="font-size: 1.5rem; font-weight: 800;">100%</div>
                    <div style="font-size: 0.75rem; font-weight: 700; opacity: 0.8;">TYPE SAFE</div>
                </div>
                <div style="height: 40px; width: 1px; background: rgba(255,255,255,0.3);"></div>
                <div style="text-align: center;">
                    <div style="font-size: 1.5rem; font-weight: 800;">BLAZING</div>
                    <div style="font-size: 0.75rem; font-weight: 700; opacity: 0.8;">FAST</div>
                </div>
            </div>
        </div>
    </div>

    <!-- Sisi Form -->
    <div class="split-side-content">
        <div class="content-container">
            <div style="margin-bottom: 3rem;">
                <h2 class="title" style="font-size: 2.8rem; margin-bottom: 0.5rem;">Daftar</h2>
                <p class="text-muted" style="font-weight: 500;">Lengkapi formulir di bawah untuk bergabung.</p>
            </div>

            <form hx-post="/register" hx-target="body" hx-push-url="true" hx-indicator="#indicator" style="display: flex; flex-direction: column; gap: 1.5rem;">
                <div>
                    <label class="form-label">Nama Lengkap</label>
                    <input type="text" name="name" class="form-control" placeholder="Nama Anda" value="{{ old.name }}" required autofocus>
                    {% if errors.name %}
                        <div style="color: var(--secondary); font-size: 0.85rem; margin-top: 0.5rem; font-weight: 600;">{{ errors.name }}</div>
                    {% endif %}
                </div>

                <div>
                    <label class="form-label">Email Address</label>
                    <input type="email" name="email" class="form-control" placeholder="nama@email.com" value="{{ old.email }}" required>
                    {% if errors.email %}
                        <div style="color: var(--secondary); font-size: 0.85rem; margin-top: 0.5rem; font-weight: 600;">{{ errors.email }}</div>
                    {% endif %}
                </div>

                <div>
                    <label class="form-label">Password</label>
                    <input type="password" name="password" class="form-control" placeholder="Min. 8 karakter" required>
                    {% if errors.password %}
                        <div style="color: var(--secondary); font-size: 0.85rem; margin-top: 0.5rem; font-weight: 600;">{{ errors.password }}</div>
                    {% endif %}
                </div>

                <div style="margin-top: 1rem;">
                    <button type="submit" class="btn btn-primary w-100" style="padding: 1.25rem;">
                        BUAT AKUN SEKARANG
                    </button>
                </div>

                <p class="text-center" style="font-size: 0.95rem; color: var(--text-muted); margin-top: 1rem;">
                    Sudah punya akun? <a href="/login" style="font-weight: 800; color: var(--accent); text-decoration: none;">Login Disini</a>
                </p>
            </form>
        </div>
    </div>
</div>
{% endblock %}
"##;

    let forgot_template = r##"{% extends "layouts/app.rb.html" %}

{% block title %}Lupa Password - RustBasic{% endblock %}

{% block content %}
<div class="split-screen">
    <!-- Sisi Visual -->
    <div class="split-side-visual" style="background: linear-gradient(135deg, var(--primary), var(--secondary));">
        <div class="visual-inner" style="max-width: 600px;">
            <div style="margin-bottom: 2rem;">
                <span class="badge" style="background: rgba(255,255,255,0.2); color: #fff; border: none;">SECURITY ASSIST</span>
            </div>
            <h1 style="font-size: 3.5rem; font-weight: 900; line-height: 1.1; margin-bottom: 1.5rem; text-shadow: 0 10px 20px rgba(0,0,0,0.1);">
                Lupa <br> <span style="color: rgba(255,255,255,0.8);">Password?</span>
            </h1>
            <p style="font-size: 1.2rem; opacity: 0.9; margin-bottom: 2.5rem; font-weight: 500;">
                Jangan khawatir, hal ini biasa terjadi. Kami akan membantu Anda mendapatkan akses kembali dengan aman.
            </p>
        </div>
    </div>

    <!-- Sisi Form -->
    <div class="split-side-content">
        <div class="content-container">
            <div style="margin-bottom: 3rem;">
                <h2 class="title" style="font-size: 2.8rem; margin-bottom: 0.5rem;">Reset</h2>
                <p class="text-muted" style="font-weight: 500;">Masukkan email Anda untuk menerima link reset.</p>
            </div>

            <form hx-post="/forgot-password" hx-target="body" hx-push-url="true" hx-indicator="#indicator" style="display: flex; flex-direction: column; gap: 1.5rem;">
                <div>
                    <label class="form-label">Email Address</label>
                    <input type="email" name="email" class="form-control" placeholder="nama@email.com" value="{{ old.email }}" required autofocus>
                    {% if errors.email %}
                        <div style="color: var(--secondary); font-size: 0.85rem; margin-top: 0.5rem; font-weight: 600;">{{ errors.email }}</div>
                    {% endif %}
                </div>

                <div style="margin-top: 1rem;">
                    <button type="submit" class="btn btn-primary w-100" style="padding: 1.25rem;">
                        KIRIM LINK RESET PASSWORD
                    </button>
                </div>

                <p class="text-center" style="font-size: 0.95rem; color: var(--text-muted); margin-top: 1rem;">
                    Ingat password Anda? <a href="/login" style="font-weight: 800; color: var(--accent); text-decoration: none;">Login Disini</a>
                </p>
            </form>
        </div>
    </div>
</div>
{% endblock %}
"##;

    let login_view = "src/resources/views/auth/login.rb.html";
    if !std::path::Path::new(login_view).exists() {
        fs::write(login_view, login_template).ok();
    }
    
    let register_view = "src/resources/views/auth/register.rb.html";
    if !std::path::Path::new(register_view).exists() {
        fs::write(register_view, register_template).ok();
    }

    let forgot_view = "src/resources/views/auth/forgot.rb.html";
    if !std::path::Path::new(forgot_view).exists() {
        fs::write(forgot_view, forgot_template).ok();
    }

    let reset_view = "src/resources/views/auth/reset.rb.html";
    if !std::path::Path::new(reset_view).exists() {
        let reset_template = r##"{% extends "layouts/app.rb.html" %}

{% block title %}Reset Password - RustBasic{% endblock %}

{% block content %}
<div class="split-screen">
    <!-- Sisi Visual -->
    <div class="split-side-visual" style="background: linear-gradient(135deg, var(--accent), var(--primary));">
        <div class="visual-inner" style="max-width: 600px;">
            <div style="margin-bottom: 2rem;">
                <span class="badge" style="background: rgba(255,255,255,0.2); color: #fff; border: none;">RECOVER ACCESS</span>
            </div>
            <h1 style="font-size: 3.5rem; font-weight: 900; line-height: 1.1; margin-bottom: 1.5rem; text-shadow: 0 10px 20px rgba(0,0,0,0.1);">
                Buat Password <br> <span style="color: rgba(255,255,255,0.8);">Baru</span>
            </h1>
            <p style="font-size: 1.2rem; opacity: 0.9; margin-bottom: 2.5rem; font-weight: 500;">
                Hampir selesai! Gunakan kombinasi password yang kuat untuk menjaga keamanan akun Anda di masa depan.
            </p>
        </div>
    </div>

    <!-- Sisi Form -->
    <div class="split-side-content">
        <div class="content-container">
            <div style="margin-bottom: 3rem;">
                <h2 class="title" style="font-size: 2.8rem; margin-bottom: 0.5rem;">Update</h2>
                <p class="text-muted" style="font-weight: 500;">Silakan masukkan password baru Anda.</p>
            </div>

            <form hx-post="/reset-password" hx-target="body" hx-push-url="true" hx-indicator="#indicator" style="display: flex; flex-direction: column; gap: 1.5rem;">
                <input type="hidden" name="token" value="{{ token }}">
                
                <div>
                    <label class="form-label">Password Baru</label>
                    <input type="password" name="password" class="form-control" placeholder="Min. 8 karakter" required autofocus>
                    {% if errors.password %}
                        <div style="color: var(--secondary); font-size: 0.85rem; margin-top: 0.5rem; font-weight: 600;">{{ errors.password }}</div>
                    {% endif %}
                </div>

                <div style="margin-top: 1rem;">
                    <button type="submit" class="btn btn-primary w-100" style="padding: 1.25rem;">
                        SIMPAN PASSWORD BARU
                    </button>
                </div>
            </form>
        </div>
    </div>
</div>
{% endblock %}
"##;
        fs::write(reset_view, reset_template).ok();
    }

    let email_reset_view = "src/resources/views/emails/reset.rb.html";
    if !std::path::Path::new(email_reset_view).exists() {
        fs::create_dir_all("src/resources/views/emails").ok();
        let email_reset_template = r##"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body { font-family: 'Inter', -apple-system, sans-serif; line-height: 1.6; color: #1a1a1a; margin: 0; padding: 0; }
        .container { max-width: 600px; margin: 0 auto; padding: 40px 20px; }
        .card { background: #ffffff; border-radius: 16px; overflow: hidden; box-shadow: 0 4px 24px rgba(0,0,0,0.06); border: 1px solid #f0f0f0; }
        .header { background: linear-gradient(135deg, #6366f1, #a855f7); padding: 40px; text-align: center; color: white; }
        .content { padding: 40px; }
        .button { display: inline-block; padding: 14px 32px; background: #6366f1; color: #ffffff !important; text-decoration: none; border-radius: 8px; font-weight: 600; margin: 24px 0; }
        .footer { padding: 24px; text-align: center; font-size: 13px; color: #6b7280; }
        h1 { margin: 0; font-size: 24px; font-weight: 800; letter-spacing: -0.025em; }
        p { margin: 16px 0; color: #4b5563; }
        .divider { height: 1px; background: #f3f4f6; margin: 24px 0; }
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <div class="header">
                <h1>{{ app_name }}</h1>
            </div>
            <div class="content">
                <h2 style="margin: 0; color: #111827; font-size: 20px;">Halo!</h2>
                <p>Anda menerima email ini karena kami menerima permintaan reset password untuk akun Anda di <strong>{{ app_name }}</strong>.</p>
                
                <div style="text-align: center;">
                    <a href="{{ reset_url }}" class="button">Reset Password Saya</a>
                </div>

                <p style="font-size: 14px; color: #9ca3af;">Link ini akan kadaluarsa dalam 60 menit. Jika Anda tidak merasa meminta reset password, abaikan saja email ini.</p>
                
                <div class="divider"></div>
                
                <p style="font-size: 12px; color: #9ca3af;">
                    Jika Anda kesulitan menekan tombol, salin dan tempel URL berikut ke browser Anda:<br>
                    <span style="word-break: break-all; color: #6366f1;">{{ reset_url }}</span>
                </p>
            </div>
        </div>
        <div class="footer">
            &copy; 2026 {{ app_name }}. All rights reserved.
        </div>
    </div>
</body>
</html>
"##;
        fs::write(email_reset_view, email_reset_template).ok();
    }

    let dashboard_view = "src/resources/views/dashboard.rb.html";
    if !std::path::Path::new(dashboard_view).exists() {
        let dashboard_template = r##"{% extends "layouts/app.rb.html" %}

{% block title %}{{ title }} - RustBasic{% endblock %}

{% block content %}
<div class="split-screen" style="background: #f8faff;">
    <!-- Sidebar / Navigation (Kiri) -->
    <div class="split-side-visual" style="flex: 0.35; align-items: flex-start; text-align: left; padding: 3rem; background: linear-gradient(180deg, var(--text-main), #2d3436);">
        <div style="width: 100%;">
            <div style="display: flex; align-items: center; gap: 1rem; margin-bottom: 3rem;">
                <div style="width: 50px; height: 50px; background: var(--primary); border-radius: 12px; display: flex; align-items: center; justify-content: center; font-weight: 900; color: white; font-size: 1.5rem;">
                    R
                </div>
                <h2 style="font-size: 1.5rem; font-weight: 800; color: white;">RustBasic</h2>
            </div>

            <div style="background: rgba(255,255,255,0.05); padding: 1.5rem; border-radius: 1.5rem; border: 1px solid rgba(255,255,255,0.1); margin-bottom: 3rem;">
                <div style="display: flex; align-items: center; gap: 1rem;">
                    <div style="width: 45px; height: 45px; background: var(--accent); border-radius: 50%; display: flex; align-items: center; justify-content: center; font-weight: 800; color: white; font-size: 1.2rem;">
                        {{ user_name[0] | upper }}
                    </div>
                    <div>
                        <div style="font-weight: 700; color: white; font-size: 0.95rem;">{{ user_name }}</div>
                        <div style="font-size: 0.8rem; color: rgba(255,255,255,0.5);">Administrator</div>
                    </div>
                </div>
            </div>

            <nav style="display: flex; flex-direction: column; gap: 0.5rem;">
                <a href="/dashboard" class="btn" style="background: var(--primary); color: white; justify-content: flex-start; text-transform: none; letter-spacing: normal; padding: 1rem 1.5rem; border-radius: 12px;">
                    📊 Dashboard Overview
                </a>
                <a href="/" class="btn" style="color: rgba(255,255,255,0.6); justify-content: flex-start; text-transform: none; letter-spacing: normal; padding: 1rem 1.5rem;">
                    🏠 Main Website
                </a>
            </nav>

            <div style="margin-top: 5rem;">
                <form hx-post="/logout" hx-target="body" style="margin:0;">
                    <button type="submit" class="btn w-100" style="background: rgba(239, 68, 68, 0.1); color: #ef4444; border: 1px solid rgba(239, 68, 68, 0.2); border-radius: 12px; font-weight: 700; padding: 1rem;">
                        🚪 KELUAR SISTEM
                    </button>
                </form>
            </div>
        </div>
    </div>

    <!-- Main Workspace (Kanan) -->
    <div class="split-side-content" style="flex: 1.2; align-items: flex-start; justify-content: flex-start; padding: 0;">
        <div style="width: 100%; padding: 4rem;">
            <header style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 4rem;">
                <div>
                    <h1 class="title" style="font-size: 2.5rem; text-align: left; margin-bottom: 0.25rem;">Overview</h1>
                    <p class="text-muted" style="font-weight: 500;">Selamat datang kembali, kendalikan project Anda.</p>
                </div>
                <div style="display: flex; gap: 1rem;">
                    <div class="badge" style="background: white; padding: 0.8rem 1.5rem; box-shadow: 0 4px 6px rgba(0,0,0,0.02);">
                        Server: <span style="color: var(--primary);">Running</span>
                    </div>
                </div>
            </header>

            <!-- Stats Grid -->
            <div style="display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 2rem; margin-bottom: 4rem;">
                <div style="background: white; border-radius: 24px; padding: 2rem; box-shadow: 0 10px 20px rgba(0,0,0,0.02); border: 1px solid rgba(0,0,0,0.03);">
                    <div style="color: var(--text-muted); font-size: 0.85rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 1.5rem;">
                        User Terdaftar
                    </div>
                    <div style="display: flex; align-items: baseline; gap: 0.5rem;">
                        <div style="font-size: 3rem; font-weight: 900; color: var(--text-main);">{{ total_users }}</div>
                        <div style="color: #10b981; font-weight: 700; font-size: 0.9rem;">↑ 12%</div>
                    </div>
                </div>

                <div style="background: white; border-radius: 24px; padding: 2rem; box-shadow: 0 10px 20px rgba(0,0,0,0.02); border: 1px solid rgba(0,0,0,0.03);">
                    <div style="color: var(--text-muted); font-size: 0.85rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 1.5rem;">
                        Response Time
                    </div>
                    <div style="display: flex; align-items: baseline; gap: 0.5rem;">
                        <div style="font-size: 3rem; font-weight: 900; color: var(--accent);">24</div>
                        <div style="color: var(--accent); font-weight: 700; font-size: 0.9rem;">ms</div>
                    </div>
                </div>

                <div style="background: white; border-radius: 24px; padding: 2rem; box-shadow: 0 10px 20px rgba(0,0,0,0.02); border: 1px solid rgba(0,0,0,0.03);">
                    <div style="color: var(--text-muted); font-size: 0.85rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 1.5rem;">
                        Database Status
                    </div>
                    <div style="display: flex; align-items: center; gap: 0.8rem; padding: 0.5rem 0;">
                        <div style="width: 12px; height: 12px; background: #10b981; border-radius: 50%; box-shadow: 0 0 10px #10b981;"></div>
                        <div style="font-size: 1.5rem; font-weight: 800; color: #10b981;">HEALTHY</div>
                    </div>
                </div>
            </div>

            <!-- Main Panel -->
            <div class="glass-panel" style="max-width: none; padding: 3rem; margin: 0; border-radius: 32px; background: linear-gradient(135deg, white, #f1f3f5);">
                <div style="display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 2rem;">
                    <div>
                        <h3 style="font-size: 1.8rem; font-weight: 800; margin-bottom: 0.5rem;">Informasi Server</h3>
                        <p class="text-muted">Detail lingkungan eksekusi RustBasic Anda.</p>
                    </div>
                    <span class="badge" style="background: var(--primary); color: white;">v2026.1</span>
                </div>
                
                <div style="background: var(--text-main); color: #00ff00; padding: 2rem; border-radius: 16px; font-family: monospace; font-size: 0.9rem; line-height: 1.6; box-shadow: inset 0 2px 10px rgba(0,0,0,0.5);">
                    <div style="color: #636e72;">// RustBasic Kernel System</div>
                    <div>[OK] Compiled with Axum 0.8.2</div>
                    <div>[OK] Database Pool: Sea-ORM Connection Established</div>
                    <div>[OK] Live Reload: Active on port 4000</div>
                    <div>[OK] Workers: 8 logical threads spawned</div>
                </div>
            </div>
        </div>
    </div>
</div>
{% endblock %}
"##;
        fs::write(dashboard_view, dashboard_template).ok();
    }
    
    // 6. Create Dashboard Controller
    let dashboard_controller_path = "src/app/http/controllers/dashboard_controller.rs";
    if !std::path::Path::new(dashboard_controller_path).exists() {
        let dashboard_template = r#"use crate::app::view;
use crate::app::models::users;
use rustbasic_core::requests::Request;
use rustbasic_core::server::AppState;
use rustbasic_core::axum::{response::IntoResponse, extract::State};
use rustbasic_core::minijinja::context;
use rustbasic_core::sea_orm::{EntityTrait, PaginatorTrait};

pub struct DashboardController;

impl DashboardController {
    pub async fn index(State(state): State<AppState>, req: Request) -> impl IntoResponse {
        let user_id = req.session.get::<i32>("user_id").unwrap_or(0);
        let user = users::Entity::find_by_id(user_id).one(&state.db).await.ok().flatten();
        let total_users = users::Entity::find().count(&state.db).await.unwrap_or(0);

        view(&req, "dashboard.rb.html", context! {
            title => "Dashboard",
            user_name => user.as_ref().map(|u| u.name.clone()).unwrap_or("Guest".to_string()),
            user_email => user.as_ref().map(|u| u.email.clone()).unwrap_or_default(),
            total_users => total_users,
        })
    }
}
"#;
        fs::write(dashboard_controller_path, dashboard_template).ok();
        println!("   {} {}", "✅ Created:".green(), dashboard_controller_path.cyan());
    }
    update_controller_mod_rs("dashboard_controller");

    println!("   {} Folder src/resources/views/auth dan dashboard siap.", "✅ Views:".green());

    // 6. Update welcome.rb.html
    let welcome_path = "src/resources/views/welcome.rb.html";
    if let Ok(content) = fs::read_to_string(welcome_path) {
        if !content.contains("{% if auth %}") {
            println!("   {} {}", "⚠️  Manual:".yellow(), "Pastikan welcome.rb.html memiliki tombol login/register.".dimmed());
        } else {
            println!("   {} {}", "✅ OK:".green(), "welcome.rb.html sudah memiliki logika auth.".dimmed());
        }
    }

    println!("\n{}", "✨ Authentication scaffolded successfully!".green().bold());
    println!("{}", "Jalankan 'cargo rustbasic route:list' untuk melihat route baru.".dimmed());
}

pub async fn remove_auth() {
    println!("\n{}", "🗑️  Removing Authentication Scaffold...".red().bold());

    // 1. Delete src/routes/auth.rs
    let auth_route_path = "src/routes/auth.rs";
    if std::path::Path::new(auth_route_path).exists() {
        fs::remove_file(auth_route_path).ok();
        println!("   {} {}", "✅ Deleted:".green(), auth_route_path.cyan());
    }

    // 2. Update src/routes/mod.rs
    let routes_mod_path = "src/routes/mod.rs";
    if let Ok(mut content) = fs::read_to_string(routes_mod_path)
        && content.contains("pub mod auth;") {
            content = content.replace("pub mod auth;\n", "");
            fs::write(routes_mod_path, content).ok();
            println!("   {} {}", "📝 Updated:".blue(), routes_mod_path.cyan());
        }

    // 3. Update src/routes/web.rs
    let web_route_path = "src/routes/web.rs";
    if let Ok(mut content) = fs::read_to_string(web_route_path) {
        let mut changed = false;
        
        // Remove imports
        if content.contains("use rustbasic_core::axum::{Router, routing::{get, post}, middleware::from_fn};") {
            content = content.replace("use rustbasic_core::axum::{Router, routing::{get, post}, middleware::from_fn};", "use rustbasic_core::axum::{Router, routing::get};");
            changed = true;
        }
        
        let imports_to_remove = [
            "use crate::app::http::controllers::{auth, dashboard_controller};\n",
            "use crate::app::http::middleware::auth::auth_middleware;\n",
            "use rustbasic_core::server::AppState;\n",
            "use crate::routes::auth as auth_routes;\n",
            "use crate::app::http::controllers::{auth, dashboard_controller};",
            "use crate::app::http::middleware::auth::auth_middleware;",
            "use crate::routes::auth as auth_routes;",
        ];
        
        for imp in imports_to_remove {
            if content.contains(imp) {
                content = content.replace(imp, "");
                changed = true;
            }
        }
        
        // Re-add server::AppState if it was removed
        if !content.contains("use rustbasic_core::server::AppState;") {
            content = content.replace("use rustbasic_core::axum::{Router, routing::get};", "use rustbasic_core::axum::{Router, routing::get};\nuse rustbasic_core::server::AppState;");
        }

        // Remove auth_protected_routes logic and restore basic Router
        if content.contains("let auth_protected_routes = Router::new()") {
            let re = Regex::new(r##"(?s)\s*let auth_protected_routes = Router::new\(\).*?\.layer\(from_fn\(auth_middleware\)\);\s*"##).unwrap();
            content = re.replace(&content, "\n").to_string();
            
            content = content.replace(".merge(auth_routes::router())", "");
            content = content.replace(".merge(auth_protected_routes)", "");
            
            // Restore clean Router::new()
            let clean_router = r#"    Router::new()
        .route("/", get(welcome_controller::index))
        .route("/dev", get(welcome_controller::dev_info))"#;
            
            let router_re = Regex::new(r##"(?s)Router::new\(\).*?\.route\(\s*\"/dev\"\s*,\s*get\(welcome_controller::dev_info\)\s*\)"##).unwrap();
            content = router_re.replace(&content, clean_router).to_string();
            
            // Final cleanup of multiple newlines
            let multi_newline_re = Regex::new(r#"\n{3,}"#).unwrap();
            content = multi_newline_re.replace_all(&content, "\n\n").to_string();
            
            changed = true;
        }

        if changed {
            fs::write(web_route_path, content).ok();
            println!("   {} {}", "📝 Updated:".blue(), web_route_path.cyan());
        }
    }

    // 7. Delete Controllers
    let auth_controller_dir = "src/app/http/controllers/auth";
    if std::path::Path::new(auth_controller_dir).exists() {
        fs::remove_dir_all(auth_controller_dir).ok();
        println!("   {} {}", "✅ Deleted:".green(), auth_controller_dir.cyan());
    }

    // 7.1 Delete Password Resets Migration & Model
    if let Ok(entries) = std::fs::read_dir("database/migrations") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str()
                && name.ends_with("_create_password_resets_table.rs") {
                    let path = entry.path();
                    fs::remove_file(&path).ok();
                    println!("   {} {}", "✅ Deleted:".green(), path.display().to_string().cyan());
                }
        }
    }
    
    let model_path = "src/app/models/password_resets.rs";
    if std::path::Path::new(model_path).exists() {
        fs::remove_file(model_path).ok();
        println!("   {} {}", "✅ Deleted:".green(), model_path.cyan());
    }

    // 8. Delete Views
    let auth_view_dir = "src/resources/views/auth";
    if std::path::Path::new(auth_view_dir).exists() {
        fs::remove_dir_all(auth_view_dir).ok();
        println!("   {} {}", "✅ Deleted:".green(), auth_view_dir.cyan());
    }

    // 8.1 Delete Auth Middleware
    let auth_middleware_path = "src/app/http/middleware/auth.rs";
    if std::path::Path::new(auth_middleware_path).exists() {
        fs::remove_file(auth_middleware_path).ok();
        println!("   {} {}", "✅ Deleted:".green(), auth_middleware_path.cyan());
    }

    let middleware_mod_path = "src/app/http/middleware/mod.rs";
    if let Ok(mut content) = fs::read_to_string(middleware_mod_path)
        && content.contains("pub mod auth;") {
            content = content.replace("pub mod auth;\n", "");
            fs::write(middleware_mod_path, content).ok();
            println!("   {} {}", "📝 Updated:".blue(), middleware_mod_path.cyan());
        }

    // 6. Delete Dashboard Controller
    let dashboard_path = "src/app/http/controllers/dashboard_controller.rs";
    if std::path::Path::new(dashboard_path).exists() {
        fs::remove_file(dashboard_path).ok();
        println!("   {} {}", "✅ Deleted:".green(), dashboard_path.cyan());
    }

    // 7. Update src/app/http/controllers/mod.rs
    let controllers_mod_path = "src/app/http/controllers/mod.rs";
    if let Ok(mut content) = fs::read_to_string(controllers_mod_path) {
        let mut changed = false;
        if content.contains("pub mod auth;") {
            content = content.replace("pub mod auth;\n", "");
            changed = true;
        }
        if content.contains("pub mod dashboard_controller;") {
            content = content.replace("pub mod dashboard_controller;\n", "");
            changed = true;
        }
        if changed {
            fs::write(controllers_mod_path, content).ok();
            println!("   {} {}", "📝 Updated:".blue(), controllers_mod_path.cyan());
        }
    }

    // 7.2 Update src/app/models/mod.rs
    let models_mod_path = "src/app/models/mod.rs";
    if let Ok(mut content) = fs::read_to_string(models_mod_path)
        && content.contains("pub mod password_resets;") {
            content = content.replace("pub mod password_resets;\n", "");
            content = content.replace("pub mod password_resets;", "");
            fs::write(models_mod_path, content).ok();
            println!("   {} {}", "📝 Updated:".blue(), models_mod_path.cyan());
        }

    // 7.3 Update database/migrations/mod.rs
    let migration_mod_path = "database/migrations/mod.rs";
    if let Ok(content) = fs::read_to_string(migration_mod_path) {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut changed = false;
        
        // Remove the mod line
        lines.retain(|line| {
            if line.contains("_create_password_resets_table;") || (line.contains("Box::new(") && line.contains("_create_password_resets_table::Migration")) {
                changed = true;
                false
            } else {
                true
            }
        });

        if changed {
            fs::write(migration_mod_path, lines.join("\n")).ok();
            println!("   {} {}", "📝 Updated:".blue(), migration_mod_path.cyan());
        }
    }

    // 7.4 Delete Migration Record from Database
    println!("   {} {}", "⏳".blue(), "Cleaning up migration records from database...".dimmed());
    let cfg = crate::Config::load();
    let db_url = if cfg.db_connection == "mysql" {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            cfg.db_username, cfg.db_password, cfg.db_host, cfg.db_port, cfg.db_database
        )
    } else {
        format!("sqlite:database/{}.sqlite?mode=rwc", cfg.db_database)
    };

    if let Ok(db) = sea_orm::Database::connect(db_url).await {
        use sea_orm::ConnectionTrait;
        let table_name = if cfg.db_connection == "mysql" { "sea_orm_migrations" } else { "seaql_migrations" };
        let sql = format!("DELETE FROM {} WHERE version LIKE '%_create_password_resets_table'", table_name);
        let _ = db.execute(sea_orm::Statement::from_string(cfg.db_backend(), sql)).await;
        println!("   {} {}", "✅ Cleaned:".green(), "Database migration records removed.".cyan());
    }

    println!("\n{}", "✨ Authentication removed successfully!".green().bold());
}
