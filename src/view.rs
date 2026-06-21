/* ---------------------------------------------------------
 * 📑 LABEL: VIEW ENGINE (config/view.rs)
 * Mengatur template engine (RustBasic Template) dan fungsi render.
 * --------------------------------------------------------- */

use crate::router::{Html, IntoResponse, Response};
use http::StatusCode;
use crate::chrono::{DateTime, FixedOffset};
use crate::chrono_tz::{self, Tz};
use crate::requests::Request as AppRequest;
use crate::Config;
use serde_json::{json, Value};
use crate::template::TemplateEngine;

use crate::tracing;

static EMBEDDED_TEMPLATES_GET: std::sync::OnceLock<fn(&str) -> Option<crate::rust_embed::EmbeddedFile>> = std::sync::OnceLock::new();

pub fn set_embedded_templates(f: fn(&str) -> Option<crate::rust_embed::EmbeddedFile>) {
    EMBEDDED_TEMPLATES_GET.set(f).ok();
}

fn load_template_content(name: &str) -> Result<String, String> {
    let cfg = Config::load();
    if cfg.app_debug {
        let path = format!("src/resources/views/{}", name);
        if let Ok(content) = std::fs::read_to_string(&path) {
            return Ok(content);
        }
    }
    
    // Fallback ke embedded templates di memori
    let file = EMBEDDED_TEMPLATES_GET.get().and_then(|f| f(name));
    if let Some(file) = file
        && let Ok(content) = std::str::from_utf8(&file.data) {
            return Ok(content.to_string());
        }
    
    Err(format!("Template '{}' tidak ditemukan", name))
}

// 3. Fungsi Helper untuk Render HTML Statis
pub fn render(template: &str, context: Value) -> Response {
    render_internal(template, context)
}

pub fn render_to_string(template: &str, context: Value) -> String {
    let content = match load_template_content(template) {
        Ok(c) => c,
        Err(e) => return format!("Template error: {}", e),
    };
    
    let mut engine = TemplateEngine::new();
    
    // Register custom filters
    engine.add_filter("diff_for_humans", |val: &Value, _args: &[Value]| {
        if let Some(value) = val.as_str()
            && let Ok(dt) = DateTime::<FixedOffset>::parse_from_rfc3339(value) {
                let now = crate::chrono::Utc::now();
                let dt_utc = dt.with_timezone(&crate::chrono::Utc);
                let duration = now.signed_duration_since(dt_utc);
                let seconds = duration.num_seconds();
                let result = if seconds < 0 {
                    let seconds = -seconds;
                    if seconds < 60 {
                        "in a few seconds".to_string()
                    } else {
                        let minutes = seconds / 60;
                        if minutes < 60 {
                            format!("in {} minute{}", minutes, if minutes > 1 { "s" } else { "" })
                        } else {
                            let hours = minutes / 60;
                            if hours < 24 {
                                format!("in {} hour{}", hours, if hours > 1 { "s" } else { "" })
                            } else {
                                let days = hours / 24;
                                if days < 30 {
                                    format!("in {} day{}", days, if days > 1 { "s" } else { "" })
                                } else {
                                    let months = days / 30;
                                    if months < 12 {
                                        format!("in {} month{}", months, if months > 1 { "s" } else { "" })
                                    } else {
                                        let years = months / 12;
                                        format!("in {} year{}", years, if years > 1 { "s" } else { "" })
                                    }
                                }
                            }
                        }
                    }
                } else {
                    if seconds < 60 {
                        "a few seconds ago".to_string()
                    } else {
                        let minutes = seconds / 60;
                        if minutes < 60 {
                            format!("{} minute{} ago", minutes, if minutes > 1 { "s" } else { "" })
                        } else {
                            let hours = minutes / 60;
                            if hours < 24 {
                                format!("{} hour{} ago", hours, if hours > 1 { "s" } else { "" })
                            } else {
                                let days = hours / 24;
                                if days < 30 {
                                    format!("{} day{} ago", days, if days > 1 { "s" } else { "" })
                                } else {
                                    let months = days / 30;
                                    if months < 12 {
                                        format!("{} month{} ago", months, if months > 1 { "s" } else { "" })
                                    } else {
                                        let years = months / 12;
                                        format!("{} year{} ago", years, if years > 1 { "s" } else { "" })
                                    }
                                }
                            }
                        }
                    }
                };
                return Value::String(result);
            }
        val.clone()
    });

    engine.add_filter("format_date", |val: &Value, args: &[Value]| {
        let fmt = args.first().and_then(|a| a.as_str()).unwrap_or("%Y-%m-%d");
        if let Some(value) = val.as_str() {
            let cfg = Config::load();
            let tz_str = cfg.app_timezone.trim();
            let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
            
            if let Ok(dt) = DateTime::<FixedOffset>::parse_from_rfc3339(value) {
                return Value::String(dt.with_timezone(&tz).format(fmt).to_string());
            }
        }
        val.clone()
    });

    engine.render(&content, &context).unwrap_or_else(|e| format!("Render error: {}", e))
}

// 4. Fungsi Helper untuk Render dengan Session
pub fn view(req: &AppRequest, template: &str, ctx: Value) -> Response {
    let mut ctx_value = ctx;
    
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

    render_internal(template, ctx_value)
}

fn render_internal(template: &str, context: Value) -> Response {
    let cfg = crate::Config::load();
    tracing::debug!("Rendering template: {} (APP_DEBUG: {})", template, cfg.app_debug);

    match load_template_content(template) {
        Ok(content) => {
            let mut engine = TemplateEngine::new();
            
            // Register custom filters
            engine.add_filter("diff_for_humans", |val: &Value, _args: &[Value]| {
                if let Some(value) = val.as_str()
                    && let Ok(dt) = DateTime::<FixedOffset>::parse_from_rfc3339(value) {
                        let now = crate::chrono::Utc::now();
                        let dt_utc = dt.with_timezone(&crate::chrono::Utc);
                        let duration = now.signed_duration_since(dt_utc);
                        let seconds = duration.num_seconds();
                        let result = if seconds < 0 {
                            let seconds = -seconds;
                            if seconds < 60 {
                                "in a few seconds".to_string()
                            } else {
                                let minutes = seconds / 60;
                                if minutes < 60 {
                                    format!("in {} minute{}", minutes, if minutes > 1 { "s" } else { "" })
                                } else {
                                    let hours = minutes / 60;
                                    if hours < 24 {
                                        format!("in {} hour{}", hours, if hours > 1 { "s" } else { "" })
                                    } else {
                                        let days = hours / 24;
                                        if days < 30 {
                                            format!("in {} day{}", days, if days > 1 { "s" } else { "" })
                                        } else {
                                            let months = days / 30;
                                            if months < 12 {
                                                format!("in {} month{}", months, if months > 1 { "s" } else { "" })
                                            } else {
                                                let years = months / 12;
                                                format!("in {} year{}", years, if years > 1 { "s" } else { "" })
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            if seconds < 60 {
                                "a few seconds ago".to_string()
                            } else {
                                let minutes = seconds / 60;
                                if minutes < 60 {
                                    format!("{} minute{} ago", minutes, if minutes > 1 { "s" } else { "" })
                                } else {
                                    let hours = minutes / 60;
                                    if hours < 24 {
                                        format!("{} hour{} ago", hours, if hours > 1 { "s" } else { "" })
                                    } else {
                                        let days = hours / 24;
                                        if days < 30 {
                                            format!("{} day{} ago", days, if days > 1 { "s" } else { "" })
                                        } else {
                                            let months = days / 30;
                                            if months < 12 {
                                                format!("{} month{} ago", months, if months > 1 { "s" } else { "" })
                                            } else {
                                                let years = months / 12;
                                                format!("{} year{} ago", years, if years > 1 { "s" } else { "" })
                                            }
                                        }
                                    }
                                }
                            }
                        };
                        return Value::String(result);
                    }
                val.clone()
            });

            engine.add_filter("format_date", |val: &Value, args: &[Value]| {
                let fmt = args.first().and_then(|a| a.as_str()).unwrap_or("%Y-%m-%d");
                if let Some(value) = val.as_str() {
                    let cfg = Config::load();
                    let tz_str = cfg.app_timezone.trim();
                    let tz: Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
                    
                    if let Ok(dt) = DateTime::<FixedOffset>::parse_from_rfc3339(value) {
                        return Value::String(dt.with_timezone(&tz).format(fmt).to_string());
                    }
                }
                val.clone()
            });

            match engine.render(&content, &context) {
                Ok(rendered) => Html(rendered).into_response(),
                Err(err) => {
                    tracing::error!("Gagal render template: {}", err);
                    if cfg.app_debug {
                        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Render Error: {}", err)).into_response();
                    }
                    (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
                }
            }
        }
        Err(err) => {
            tracing::error!("Template tidak ditemukan: {}", err);
            if cfg.app_debug {
                return (StatusCode::NOT_FOUND, format!("Template Not Found: {}", err)).into_response();
            }
            (StatusCode::NOT_FOUND, "Not Found").into_response()
        }
    }
}
