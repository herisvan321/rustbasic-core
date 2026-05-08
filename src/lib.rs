/* ---------------------------------------------------------
 * 📑 LABEL: CONFIG MANAGER (config/mod.rs)
 * File ini mengelola sub-modul konfigurasi dan re-export.
 * --------------------------------------------------------- */

pub mod app;
pub mod session;
pub mod database;
pub mod server;
pub mod logger;
pub mod requests;
pub mod responses;
pub mod view;
pub mod session_manager;
pub mod errors;
pub mod mail;
pub mod seeder;
pub mod cli;

// Re-export Config agar bisa dipanggil dengan crate::Config
pub use app::Config;
pub use server::AppState;
pub use requests::Request;
pub use responses::ResponseHelper;

// Re-export common Axum types
pub use axum::{Router, middleware, response::IntoResponse};
