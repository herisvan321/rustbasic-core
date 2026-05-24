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
pub mod middleware;
pub mod schema;
pub mod macros;

pub use schema::{Schema, Blueprint, ColumnBuilder};

// Re-export Config agar bisa dipanggil dengan crate::Config
pub use app::Config;
pub use server::AppState;
pub use requests::Request;
pub use responses::ResponseHelper;

// --- RE-EXPORTS ---
pub use axum;
pub use sea_orm;
pub use sqlx;
pub use tokio;
pub use tower;
pub use tower_http;
pub use minijinja;
pub use serde;
pub use serde_json;
pub use chrono;
pub use chrono_tz;
pub use chrono_humanize;
pub use dotenvy;
pub use tracing;
pub use tracing_subscriber;
pub use bcrypt;
pub use validator;
pub use uuid;
pub use async_trait::async_trait;
pub use lettre;
pub use sea_orm_migration;
pub use axum_session;
pub use colored;
pub use regex;
pub use rand;
pub use base64;
pub use dashmap;
pub use once_cell;
pub use tower_livereload;
pub use rust_embed;

pub type Router = axum::Router;
