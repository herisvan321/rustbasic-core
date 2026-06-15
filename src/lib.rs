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
#[cfg(feature = "mail")]
pub mod mail;
pub mod template;
pub mod sql;
pub mod rand;
pub mod bcrypt;


pub use testing::{TestClient, TestResponse};
pub use schema::{Schema, Blueprint, ColumnBuilder, SchemaManager, MigrationTrait, MigratorTrait, DbErr};
pub use support::{Log, Str, Validator};
#[cfg(feature = "websocket")]
pub use support::Broadcaster;
#[cfg(feature = "http-client")]
pub use support::{Http, PendingRequest, HttpResponse};
pub use database::{DB, QueryBuilder};
#[cfg(feature = "mail")]
pub use mail::{MailService, Mailer};

// Re-export Config agar bisa dipanggil dengan crate::Config
pub use app::Config;
pub use server::AppState;
pub use requests::Request;
pub use responses::ResponseHelper;
pub use router::{Router, Response, IntoResponse, State, Html, Json, Redirect, get, post, put, patch, delete};
pub use middleware::{from_fn, Next};
pub use rustbasic_core_macro::async_trait;

pub use http;
pub use tokio;
pub use template as rustbasic_template;
pub use serde;
pub use serde_json;
pub use regex;
pub mod chrono;
pub mod chrono_tz;
pub mod rust_embed;
#[cfg(feature = "http-client")]
pub use reqwest;
#[cfg(feature = "jwt")]
pub use jsonwebtoken;
#[cfg(feature = "image-processing")]
pub use image;
#[cfg(feature = "image-processing")]
pub use webp;
#[cfg(feature = "image-processing")]
pub use sha2;

#[cfg(feature = "android")]
pub use jni;
#[cfg(feature = "android")]
pub use libc;

#[cfg(feature = "desktop")]
pub use wry;
