/* ---------------------------------------------------------
 * 📑 LABEL: VIEW ENGINE (config/view.rs)
 * Mengatur template engine (Minijinja) dan fungsi render.
 * --------------------------------------------------------- */

use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use minijinja::Environment;
use chrono::DateTime;
use chrono_humanize::HumanTime;
use chrono_tz::Tz;
use std::sync::LazyLock;
use crate::requests::Request as AppRequest;
use crate::Config;
use serde_json::{json, Value};
use regex::Regex;

// 1. Load Static Assets into Memory
static HTMX_SRC: LazyLock<String> = LazyLock::new(|| {
    include_str!("../resources/js/htmx.min.js").to_string()
});

static CSS_SRC: LazyLock<String> = LazyLock::new(|| {
    include_str!("../resources/css/style.css").to_string()
});


static EMBEDDED_TEMPLATES_GET: std::sync::OnceLock<fn(&str) -> Option<rust_embed::EmbeddedFile>> = std::sync::OnceLock::new();

pub fn set_embedded_templates(f: fn(&str) -> Option<rust_embed::EmbeddedFile>) {
    EMBEDDED_TEMPLATES_GET.set(f).ok();
}

// 2. Setup Engine Template (Minijinja)
pub static JINJA: LazyLock<Environment<'static>> = LazyLock::new(|| {
    let mut env = Environment::new();
    
    // Default Loader: Mencari di disk (jika debug), lalu fallback ke memori
    env.set_loader(|name| {
        let cfg = Config::load();
        if cfg.app_debug {
            let path = format!("src/resources/views/{}", name);
            if let Ok(content) = std::fs::read_to_string(&path) {
                return Ok(Some(content));
            }
        }
        
        // Fallback ke embedded templates di memori
        let file = EMBEDDED_TEMPLATES_GET.get().and_then(|f| f(name));
        if let Some(file) = file {
            if let Ok(content) = std::str::from_utf8(&file.data) {
                return Ok(Some(content.to_string()));
            }
        }
        
        Ok(None)
    });

    // --- REGISTER CARBON-LIKE FILTERS ---

    // Filter: {{ date | diff_for_humans }}
    env.add_filter("diff_for_humans", |value: String| -> String {
        if let Ok(dt) = DateTime::parse_from_rfc3339(&value) {
             let ht = HumanTime::from(dt);
             return ht.to_string();
        }
        value
    });

    // Filter: {{ date | format_date("%d %b %Y") }}
    env.add_filter("format_date", |value: String, fmt: String| -> String {
        let cfg = Config::load();
        let tz_str = cfg.app_timezone.trim();
        let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
        
        if let Ok(dt) = DateTime::parse_from_rfc3339(&value) {
             return dt.with_timezone(&tz).format(&fmt).to_string();
        }
        value
    });

    // Global Function: {{ now() }}
    env.add_function("now", || -> String {
        let cfg = Config::load();
        let tz_str = cfg.app_timezone.trim();
        let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
        
        chrono::Utc::now().with_timezone(&tz).to_rfc3339()
    });

    // Global Function: {{ htmx_js() }}
    env.add_function("htmx_js", || -> String {
        HTMX_SRC.clone()
    });

    // Global Function: {{ app_css() }}
    env.add_function("app_css", || -> String {
        CSS_SRC.clone()
    });

    env
});

// 3. Fungsi Helper untuk Render HTML Statis
pub fn render(template: &str, context: minijinja::Value) -> Response {
    render_internal(template, context)
}

pub fn render_to_string(template: &str, context: minijinja::Value) -> String {
    match JINJA.get_template(template) {
        Ok(tmpl) => tmpl.render(context).unwrap_or_else(|e| format!("Render error: {}", e)),
        Err(e) => format!("Template error: {}", e),
    }
}

// 4. Fungsi Helper untuk Render dengan Session
pub fn view(req: &AppRequest, template: &str, ctx: minijinja::Value) -> Response {
    let mut ctx_value = serde_json::to_value(&ctx).unwrap_or_else(|_| json!({}));
    
    if !ctx_value.is_object() {
        ctx_value = json!({});
    }
    
    let obj = ctx_value.as_object_mut().unwrap();

    // Default keys for flash messages
    if !obj.contains_key("errors") { obj.insert("errors".to_string(), json!({})); }
    if !obj.contains_key("old") { obj.insert("old".to_string(), json!({})); }
    if !obj.contains_key("flash_success") { obj.insert("flash_success".to_string(), json!("")); }
    if !obj.contains_key("flash_error") { obj.insert("flash_error".to_string(), json!("")); }

    if let Some(success) = req.session.get::<String>("flash_success") {
        obj.insert("flash_success".to_string(), json!(success));
        req.session.remove("flash_success");
    }
    if let Some(error) = req.session.get::<String>("flash_error") {
        obj.insert("flash_error".to_string(), json!(error));
        req.session.remove("flash_error");
    }
    if let Some(errors) = req.session.get::<Value>("errors") {
        obj.insert("errors".to_string(), errors);
        req.session.remove("errors");
    }
    if let Some(old) = req.session.get::<Value>("old_input") {
        obj.insert("old".to_string(), old);
    }

    if let Some(token) = req.session.get::<String>("_token") {
        obj.insert("csrf_token".to_string(), json!(token));
    }

    let is_logged_in = req.session.get::<i64>("user_id").is_some();
    obj.insert("auth".to_string(), json!(is_logged_in));

    render_internal(template, minijinja::Value::from_serialize(obj))
}

fn render_internal(template: &str, context: minijinja::Value) -> Response {
    let cfg = crate::Config::load();
    tracing::debug!("Rendering template: {} (APP_DEBUG: {})", template, cfg.app_debug);

    match JINJA.get_template(template) {
        Ok(tmpl) => match tmpl.render(context.clone()) {
            Ok(rendered) => {
                // --- LOGIKA MINIFIKASI ---
                let re_comments = Regex::new(r"(?s)<!--.*?-->").unwrap();
                let without_comments = re_comments.replace_all(&rendered, "");
                
                let minified = without_comments
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                
                Html(minified).into_response()
            },
            Err(err) => {
                tracing::error!("Gagal render template: {}", err);
                
                if cfg.app_debug {
                    return (StatusCode::INTERNAL_SERVER_ERROR, format!("Render Error: {}", err)).into_response();
                }

                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        },
        Err(err) => {
            tracing::error!("Template tidak ditemukan: {}", err);

            if cfg.app_debug {
                return (StatusCode::NOT_FOUND, format!("Template Not Found: {}", err)).into_response();
            }

            (StatusCode::NOT_FOUND, "Not Found").into_response()
        }
    }
}
