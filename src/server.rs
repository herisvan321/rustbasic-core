use crate::app::Config;
use crate::tracing;
use crate::session_manager::RustBasicSessionStore;
use crate::router::{Router, Response};
use crate::requests::Request;
use std::net::SocketAddr;
use crate::sql::AnyPool;
use std::sync::Arc;
use std::convert::Infallible;
use tokio::net::TcpListener;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use hyper::server::conn::http1;
use crate::rand::distr::SampleString;
#[cfg(feature = "websocket")]
use futures_util::{SinkExt, StreamExt};
#[cfg(feature = "websocket")]
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone)]
pub struct AppState {
    pub db: AnyPool,
    pub config: Arc<Config>,
}

static EMBEDDED_PUBLIC_GET: std::sync::OnceLock<fn(&str) -> Option<crate::rust_embed::EmbeddedFile>> = std::sync::OnceLock::new();

pub fn set_embedded_public(f: fn(&str) -> Option<crate::rust_embed::EmbeddedFile>) {
    EMBEDDED_PUBLIC_GET.set(f).ok();
}

fn guess_mime(path: &str) -> &'static str {
    if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".woff") {
        "font/woff"
    } else if path.ends_with(".woff2") {
        "font/woff2"
    } else {
        "application/octet-stream"
    }
}

pub async fn start_server(
    cfg: Config, 
    session_store: RustBasicSessionStore,
    db: AnyPool,
    app_router: Router<AppState>,
) {
    // Populate named routes
    let mut routes_map = std::collections::HashMap::new();
    for r in &app_router.routes {
        if let Some(ref name) = r.name {
            routes_map.insert(name.clone(), r.path.clone());
        }
    }
    let _ = crate::router::NAMED_ROUTES.set(routes_map);

    // 0. Kill port jika sedang digunakan (Force Restart)
    kill_port_if_in_use(cfg.app_port);

    // 0.5 Set Timezone Global
    unsafe {
        std::env::set_var("TZ", &cfg.app_timezone);
    }

    // 1. Inisialisasi State
    let state = AppState {
        db,
        config: Arc::new(cfg.clone()),
    };

    // 3. Tentukan Alamat
    let addr_str = format!("{}:{}", cfg.app_host, cfg.app_port);
    let addr: SocketAddr = addr_str.parse().expect("Alamat server tidak valid");
    
    tracing::info!("{} berjalan di: http://{}", cfg.app_name, addr);
    tracing::info!("WebSockets enabled: {}", cfg.websocket_enabled);
    
    // 4. Jalankan Server
    let listener = TcpListener::bind(addr).await.unwrap();
    
    loop {
        let (stream, peer_addr) = match listener.accept().await {
            Ok(ok) => ok,
            Err(_) => continue,
        };
        
        // Optimasi latensi: Kirim paket TCP langsung tanpa buffering (disable Nagle's algorithm)
        let _ = stream.set_nodelay(true);
        
        let io = TokioIo::new(stream);
        let state = state.clone();
        let router = app_router.clone();
        let peer_ip = peer_addr.ip().to_string();
        let session_store = session_store.clone();

        tokio::task::spawn(async move {
            let service = service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                let state = state.clone();
                let router = router.clone();
                let peer_ip = peer_ip.clone();
                let session_store = session_store.clone();
                async move {
                    let res = handle_http_request(req, peer_ip, state, router, session_store).await;
                    Ok::<_, Infallible>(res)
                }
            });

            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service)
                .with_upgrades()
                .await
            {
                tracing::debug!("Error serving connection: {:?}", err);
            }
        });
    }
}

pub(crate) fn match_path(route_path: &str, req_path: &str) -> bool {
    let r_parts: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();
    let q_parts: Vec<&str> = req_path.split('/').filter(|s| !s.is_empty()).collect();
    
    if r_parts.len() != q_parts.len() {
        return false;
    }
    
    for (r, q) in r_parts.iter().zip(q_parts.iter()) {
        if r.starts_with(':') || (r.starts_with('{') && r.ends_with('}')) {
            continue;
        }
        if r != q {
            return false;
        }
    }
    true
}

/// Ekstrak nilai route parameter dari URL request.
/// Contoh: route="/user/{id}", path="/user/42" → {"id": "42"}
pub(crate) fn extract_params(route_path: &str, req_path: &str) -> std::collections::HashMap<String, String> {
    let mut params = std::collections::HashMap::new();
    let r_parts: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();
    let q_parts: Vec<&str> = req_path.split('/').filter(|s| !s.is_empty()).collect();

    for (r, q) in r_parts.iter().zip(q_parts.iter()) {
        if r.starts_with('{') && r.ends_with('}') {
            // Sintaks {param}
            let key = &r[1..r.len() - 1];
            params.insert(key.to_string(), q.to_string());
        } else if r.starts_with(':') {
            // Sintaks :param
            let key = &r[1..];
            params.insert(key.to_string(), q.to_string());
        }
    }
    params
}

async fn serve_static_or_404(path: &str, state: &AppState) -> Response {
    let clean_path = path.trim_start_matches('/');
    let file_path = if clean_path.is_empty() { "index.html" } else { clean_path };

    if state.config.app_debug {
        let disk_path = std::path::Path::new("public").join(file_path);
        if disk_path.exists() && disk_path.is_file() {
            if let Ok(content) = std::fs::read(&disk_path) {
                let mime = guess_mime(file_path);
                return http::Response::builder()
                    .header(http::header::CONTENT_TYPE, mime)
                    .body(content)
                    .unwrap();
            }
        }
    } else {
        if let Some(file) = EMBEDDED_PUBLIC_GET.get().and_then(|f| f(file_path)) {
            let mime = guess_mime(file_path);
            return http::Response::builder()
                .header(http::header::CONTENT_TYPE, mime)
                .body(file.data.to_vec())
                .unwrap();
        }
    }

    crate::errors::ErrorController::not_found().await
}

async fn handle_http_request(
    #[allow(unused_mut)] mut hyper_req: hyper::Request<hyper::body::Incoming>,
    peer_ip: String,
    state: AppState,
    router: Router<AppState>,
    session_store: RustBasicSessionStore,
) -> hyper::Response<http_body_util::Full<hyper::body::Bytes>> {
    use http_body_util::BodyExt;
    
    // Check for WebSocket upgrade at "/ws" route
    let path_str = hyper_req.uri().path().to_string();
    if path_str == "/ws" {
        #[cfg(feature = "websocket")]
        {
            if !state.config.websocket_enabled {
                return hyper::Response::builder()
                    .status(http::StatusCode::NOT_FOUND)
                    .body(http_body_util::Full::new(hyper::body::Bytes::from("WebSockets are disabled")))
                    .unwrap();
            }

            if hyper_tungstenite::is_upgrade_request(&hyper_req) {
                match hyper_tungstenite::upgrade(&mut hyper_req, None) {
                    Ok((response, websocket)) => {
                        tokio::spawn(async move {
                            handle_websocket_connection(websocket).await;
                        });
                        let (parts, _) = response.into_parts();
                        return hyper::Response::from_parts(parts, http_body_util::Full::new(hyper::body::Bytes::new()));
                    }
                    Err(e) => {
                        tracing::error!("Gagal mengupgrade koneksi WebSocket: {:?}", e);
                    }
                }
            }
        }
        #[cfg(not(feature = "websocket"))]
        {
            return hyper::Response::builder()
                .status(http::StatusCode::NOT_FOUND)
                .body(http_body_util::Full::new(hyper::body::Bytes::from("WebSockets feature not compiled")))
                .unwrap();
        }
    }
    
    let (parts, body) = hyper_req.into_parts();
    let method = parts.method.clone();
    let uri = parts.uri.clone();
    let path = uri.path().to_string();
    
    let mut headers = std::collections::HashMap::new();
    for (name, val) in parts.headers.iter() {
        if let Ok(val_str) = val.to_str() {
            headers.insert(name.as_str().to_lowercase(), val_str.to_string());
        }
    }
    
    let mut inputs = serde_json::json!({});
    if let Some(query) = uri.query() {
        if let Ok(params) = crate::serde_urlencoded::from_str::<std::collections::HashMap<String, String>>(query) {
            for (k, v) in params {
                inputs[k] = serde_json::json!(v);
            }
        }
    }
    
    let body_bytes = body.collect().await.map(|c| c.to_bytes()).unwrap_or_default();
    let content_type = headers.get("content-type").map(|s| s.as_str()).unwrap_or("");
    if content_type.starts_with("application/json") {
        if let Ok(json_val) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
            if let serde_json::Value::Object(obj) = json_val {
                for (k, v) in obj {
                    inputs[k] = v;
                }
            }
        }
    } else if content_type.starts_with("application/x-www-form-urlencoded") {
        if let Ok(params) = crate::serde_urlencoded::from_bytes::<std::collections::HashMap<String, String>>(&body_bytes) {
            for (k, v) in params {
                inputs[k] = serde_json::json!(v);
            }
        }
    }
    
    let mut session_id = None;
    if let Some(cookie_header) = headers.get("cookie") {
        for cookie in cookie_header.split(';') {
            let parts: Vec<&str> = cookie.split('=').map(|s| s.trim()).collect();
            if parts.len() == 2 && parts[0] == "rustbasic_session" {
                session_id = Some(parts[1].to_string());
                break;
            }
        }
    }
    
    let id = session_id.unwrap_or_else(|| {
        crate::rand::distr::Alphanumeric.sample_string(&mut crate::rand::rng(), 40)
    });
    
    let session_data = if let Some(payload_str) = session_store.load(&id).await {
        serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&payload_str).unwrap_or_default()
    } else {
        serde_json::Map::new()
    };
    
    let session = crate::session::Session::new(id.clone());
    *session.data.lock().unwrap() = session_data;
    
    if session.get::<String>("_token").is_none() {
        let new_token = crate::rand::distr::Alphanumeric.sample_string(&mut crate::rand::rng(), 40);
        session.set("_token", new_token);
    }
    
    let req = Request {
        inputs,
        method: method.clone(),
        path: path.clone(),
        headers,
        session: session.clone(),
        state: state.clone(),
        ip_address: peer_ip,
        params: std::collections::HashMap::new(), // diisi oleh RouteDispatcher saat match
    };
    
    struct RouteDispatcher {
        router: Router<AppState>,
        state: AppState,
    }

    #[crate::async_trait]
    impl crate::router::ErasedHandler for RouteDispatcher {
        async fn call(&self, req: Request) -> Response {
            let method = req.method.clone();
            let path = req.path.clone();
            
            let mut matched_handler = None;
            let mut matched_params = std::collections::HashMap::new();
            for route in &self.router.routes {
                if match_path(&route.path, &path) {
                    for (m, h) in &route.handlers {
                        if m == &method {
                            matched_handler = Some(h.clone());
                            matched_params = extract_params(&route.path, &path);
                            break;
                        }
                    }
                }
                if matched_handler.is_some() {
                    break;
                }
            }
            
            if let Some(handler) = matched_handler {
                // Inject route params ke request
                let mut req = req;
                req.params = matched_params;
                let mut chain = std::sync::Arc::new(crate::middleware::MiddlewareChain::End(handler));
                for mw in self.router.middlewares.iter().rev() {
                    chain = std::sync::Arc::new(crate::middleware::MiddlewareChain::Next(mw.clone(), chain));
                }
                chain.next(req).await
            } else {
                serve_static_or_404(&path, &self.state).await
            }
        }
    }

    let dispatcher = std::sync::Arc::new(RouteDispatcher {
        router,
        state: state.clone(),
    });

    let mut chain = std::sync::Arc::new(crate::middleware::MiddlewareChain::End(dispatcher));
    chain = std::sync::Arc::new(crate::middleware::MiddlewareChain::Next(
        crate::middleware::from_fn(crate::middleware::security_headers::security_headers_middleware),
        chain,
    ));
    chain = std::sync::Arc::new(crate::middleware::MiddlewareChain::Next(
        crate::middleware::from_fn(crate::middleware::logging::logging_middleware),
        chain,
    ));

    let ip = req.ip_address.clone();
    let res = chain.next(req).await;
    
    let final_session_data = session.data.lock().unwrap().clone();
    if let Ok(session_json) = serde_json::to_string(&final_session_data) {
        session_store.store(&id, &session_json, &ip).await;
    }
    
    let (mut res_parts, res_body) = res.into_parts();
    let cookie_val = format!("rustbasic_session={}; Path=/; HttpOnly; SameSite=Lax", id);
    res_parts.headers.insert(
        http::header::SET_COOKIE,
        http::HeaderValue::from_str(&cookie_val).unwrap(),
    );
    
    hyper::Response::from_parts(res_parts, http_body_util::Full::new(hyper::body::Bytes::from(res_body)))
}

fn kill_port_if_in_use(_port: u16) {
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    use std::process::Command;
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    let port = _port;
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("lsof")
            .arg("-t")
            .arg(format!("-i:{}", port))
            .output();

        if let Ok(out) = output {
            let pid_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !pid_str.is_empty() {
                tracing::warn!("Port {} sedang digunakan oleh PID {}. Membunuh proses...", port, pid_str);
                
                for pid in pid_str.split('\n') {
                    if !pid.is_empty() {
                        let _ = Command::new("kill")
                            .arg("-9")
                            .arg(pid)
                            .output();
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("fuser")
            .arg("-k")
            .arg(format!("{}/tcp", port))
            .output();
    }

    #[cfg(target_os = "windows")]
    {
        let output = Command::new("cmd")
            .args(&["/C", &format!("netstat -ano | findstr :{}", port)])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut found = false;
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(pid) = parts.last() {
                    if pid.parse::<u32>().is_ok() {
                        tracing::warn!("Port {} sedang digunakan oleh PID {}. Membunuh proses...", port, pid);
                        let _ = Command::new("taskkill")
                            .args(&["/F", "/PID", pid])
                            .output();
                        found = true;
                    }
                }
            }
            if found {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }
}

#[cfg(feature = "websocket")]
async fn handle_websocket_connection(ws_stream: hyper_tungstenite::HyperWebsocket) {
    let mut ws = match ws_stream.await {
        Ok(w) => w,
        Err(e) => {
            crate::tracing::error!("WebSocket handshake failed: {:?}", e);
            return;
        }
    };

    let state = crate::support::broadcaster::Broadcaster::state();
    let conn_id = state.next_conn_id();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let mut subscribed_channels = Vec::new();

    loop {
        tokio::select! {
            incoming = ws.next() => {
                let msg = match incoming {
                    Some(Ok(m)) => m,
                    Some(Err(_)) | None => break,
                };

                if msg.is_text() {
                    let text = msg.to_text().unwrap_or("");
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
                        if let Some(action) = val.get("action").and_then(|a| a.as_str()) {
                            if let Some(channel) = val.get("channel").and_then(|c| c.as_str()) {
                                match action {
                                    "subscribe" => {
                                        let session = crate::support::broadcaster::ClientSession {
                                            id: conn_id,
                                            tx: tx.clone(),
                                        };
                                        state.subscribe(channel, session).await;
                                        subscribed_channels.push(channel.to_string());
                                    }
                                    "unsubscribe" => {
                                        state.unsubscribe(channel, conn_id).await;
                                        subscribed_channels.retain(|c| c != channel);
                                    }
                                    "broadcast" => {
                                        if let Some(event) = val.get("event").and_then(|e| e.as_str()) {
                                            if let Some(data) = val.get("data") {
                                                let msg = serde_json::json!({
                                                    "event": event,
                                                    "channel": channel,
                                                    "data": data
                                                });
                                                if let Ok(msg_str) = serde_json::to_string(&msg) {
                                                    let channels = state.channels.read().await;
                                                    if let Some(sessions) = channels.get(channel) {
                                                        for session in sessions {
                                                            if session.id != conn_id {
                                                                let _ = session.tx.send(msg_str.clone());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                } else if msg.is_close() {
                    break;
                }
            }
            outgoing = rx.recv() => {
                let text = match outgoing {
                    Some(t) => t,
                    None => break,
                };
                if ws.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
        }
    }

    for channel in subscribed_channels {
        state.unsubscribe(&channel, conn_id).await;
    }
}
