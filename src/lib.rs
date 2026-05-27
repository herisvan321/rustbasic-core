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
pub mod seeder;
pub mod middleware;
pub mod schema;
pub mod macros;
pub mod router;
pub mod support;
pub mod tracing;
pub mod uuid;
pub mod validator;
pub mod dotenvy;
pub mod colored;
pub mod base64;
pub mod serde_urlencoded;
pub mod testing;
pub mod mail;

pub use testing::{TestClient, TestResponse};
pub use schema::{Schema, Blueprint, ColumnBuilder, SchemaManager, MigrationTrait, MigratorTrait, DbErr};
pub use support::{Log, Str, Validator, Http, PendingRequest, HttpResponse};
pub use database::{DB, QueryBuilder};
pub use mail::{MailService, Mailer};

// Re-export Config agar bisa dipanggil dengan crate::Config
pub use app::Config;
pub use server::AppState;
pub use requests::Request;
pub use responses::ResponseHelper;
pub use router::{Router, Response, IntoResponse, State, Html, Json, Redirect, get, post, put, patch, delete};
pub use middleware::{from_fn, Next};
pub use rustbasic_core_macro::async_trait;

// --- RE-EXPORTS ---
pub use http;
pub use sqlx;
pub use tokio;
pub use minijinja;
pub use serde;
pub use serde_json;
pub use chrono;
pub use chrono_tz;
pub use bcrypt;

pub use rand;
pub use rust_embed;
pub use reqwest;
