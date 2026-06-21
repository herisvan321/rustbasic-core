use crate::requests::Request;
use crate::{IntoResponse, Response};
use crate::http::{header, StatusCode, HeaderValue};
use crate::serde_json::{json, Value};
use std::fs;

/// Helper untuk merender halaman SPA menggunakan React.js + Inertia.js
pub fn inertia(req: &Request, component: &str, props: Value) -> Response {
    let is_inertia = req.headers.get("x-inertia").map(|v| v == "true").unwrap_or(false);
    let url = req.path.clone();
    
    // Versi asset (bisa dikonfigurasi untuk deteksi kadaluwarsa aset)
    let version = ""; 

    let errors: std::collections::HashMap<String, String> = req.session.get("errors").unwrap_or_default();
    req.session.remove("errors");

    let success: Option<String> = req.session.get("success");
    req.session.remove("success");

    let error: Option<String> = req.session.get("error");
    req.session.remove("error");

    let warning: Option<String> = req.session.get("warning");
    req.session.remove("warning");

    let info: Option<String> = req.session.get("info");
    req.session.remove("info");

    let mut props = props;
    if let Value::Object(ref mut map) = props {
        map.insert("errors".to_string(), json!(errors));
        map.insert("flash".to_string(), json!({
            "success": success,
            "error": error,
            "warning": warning,
            "info": info
        }));
        let named_routes = crate::router::get_named_routes();
        map.insert("routes".to_string(), json!(named_routes));
        let cfg = crate::Config::load();
        map.insert("app_url".to_string(), json!(cfg.app_url));
    }

    let page_object = json!({
        "component": component,
        "props": props,
        "url": url,
        "version": version
    });

    if is_inertia {
        // Return JSON response untuk navigasi SPA Inertia
        let body = crate::serde_json::to_string(&page_object).unwrap_or_default();
        crate::http::Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .header("X-Inertia", "true")
            .header(header::VARY, "X-Inertia")
            .body(body.into_bytes())
            .unwrap()
            .into_response()
    } else {
        // Return layout root HTML "app.rb.html" untuk initial page load
        let vite_assets = get_vite_assets(req);
        let ctx = crate::serde_json::json!({
            "page": page_object,
            "vite_assets": vite_assets,
        });
        
        let mut response = crate::view::view(req, "app.rb.html", ctx).into_response();
        response.headers_mut().insert(
            header::VARY,
            HeaderValue::from_static("X-Inertia"),
        );
        response
    }
}

/// Helper untuk mendapatkan HTML tag asset Vite (JS/CSS) secara dinamis
pub fn get_vite_assets(req: &Request) -> String {
    let cfg = crate::Config::load();
    let debug = cfg.app_debug;

    if debug {
        let port = cfg.vite_port;
        // Deteksi host secara dinamis dari header request 'host' agar support beda device (misal HP)
        let mut display_host = "localhost".to_string();
        if let Some(host_hdr) = req.headers.get("host") {
            let parts: Vec<&str> = host_hdr.split(':').collect();
            if !parts.is_empty() {
                let ip_or_domain = parts[0];
                if ip_or_domain != "localhost" && ip_or_domain != "127.0.0.1" && !ip_or_domain.is_empty() {
                    display_host = ip_or_domain.to_string();
                }
            }
        }
        
        if display_host == "localhost" {
            let host = &cfg.app_host;
            if host != "0.0.0.0" && !host.is_empty() {
                display_host = host.clone();
            }
        }

        // Mode Development: Hubungkan ke Vite Dev Server kustom host dan port
        format!(
            r#"
        <!-- Vite Dev Server Integration -->
         <script type="module">
          import RefreshRuntime from 'http://{host}:{port}/@react-refresh';
          RefreshRuntime.injectIntoGlobalHook(window);
          window.$RefreshReg$ = () => {{}};
          window.$RefreshSig$ = () => (type) => type;
          window.__vite_plugin_react_preamble_installed__ = true;
        </script>
        <script type="module" src="http://{host}:{port}/src/resources/js/main.tsx"></script>
        "#,
            host = display_host,
            port = port
        )
    } else {
        // Mode Production: Baca manifest.json dari build hasil compile Vite
        let mut manifest_content = String::new();
        let paths = ["src/dist/.vite/manifest.json", "src/dist/manifest.json"];
        for path in &paths {
            if let Ok(content) = fs::read_to_string(path) {
                manifest_content = content;
                break;
            }
        }
        
        // Fallback ke EmbeddedPublic jika file di disk tidak ditemukan (misal: production standalone binary)
        if manifest_content.is_empty() {
            if let Some(f) = crate::server::get_embedded_public_fn() {
                if let Some(file) = f(".vite/manifest.json")
                    && let Ok(content) = String::from_utf8(file.data.to_vec()) {
                    manifest_content = content;
                } else if let Some(file) = f("manifest.json")
                    && let Ok(content) = String::from_utf8(file.data.to_vec()) {
                    manifest_content = content;
                }
            }
        }

        if !manifest_content.is_empty()
            && let Ok(manifest) = crate::serde_json::from_str::<Value>(&manifest_content)
            && let Some(entry) = manifest.get("src/resources/js/main.tsx") {
                let file = entry.get("file").and_then(|f| f.as_str()).unwrap_or("assets/main.js");
                let mut assets_html = format!(r#"<script type="module" src="/{}"></script>"#, file);
                
                if let Some(css_arr) = entry.get("css").and_then(|c| c.as_array()) {
                    for css in css_arr {
                        if let Some(css_str) = css.as_str() {
                            assets_html = format!(r#"<link rel="stylesheet" href="/{}" />"#, css_str) + &assets_html;
                        }
                    }
                }
                return assets_html;
        }
        
        // Fallback jika manifest.json tidak ditemukan
        r#"<script type="module" src="/assets/main.js"></script>"#.to_string()
    }
}
