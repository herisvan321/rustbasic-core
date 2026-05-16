/* ---------------------------------------------------------
 * 📑 LABEL: LOGGING MIDDLEWARE
 * Mencatat setiap request yang masuk beserta IP pengunjung.
 * Juga mencatat IP ke dalam tracker sesi untuk keamanan database.
 * --------------------------------------------------------- */

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::Request,
    middleware::Next,
    response::Response,
};
use colored::*;
use axum_session::Session;
use crate::session_manager::{RustBasicSessionStore, IP_TRACKER};
use std::net::SocketAddr;

pub async fn logging_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    session: Session<RustBasicSessionStore>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let ip = addr.ip().to_string();

    // 1. Simpan IP ke tracker
    IP_TRACKER.insert(session.get_session_id().to_string(), ip.clone());

    // 2. Log ke Terminal (Format: [HTTP] TIMESTAMP METHOD PATH from IP)
    let method_str = method.as_str();
    let method_colored = match method_str {
        "GET" => method_str.green(),
        "POST" => method_str.blue(),
        "PUT" => method_str.yellow(),
        "DELETE" => method_str.red(),
        _ => method_str.white(),
    };

    println!(
        "[{}] {} {:<6} {} from {}",
        "HTTP".magenta().bold(),
        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string().dimmed(),
        method_colored.bold(),
        path.cyan(),
        ip.yellow()
    );

    // 3. Log ke File (Tanpa warna via tracing)
    tracing::info!(method = %method, path = %path, ip = %ip, "Request");

    next.run(req).await
}
