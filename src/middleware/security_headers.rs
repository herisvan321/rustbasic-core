/* ---------------------------------------------------------
 * 📑 LABEL: SECURITY HEADERS
 * Menambahkan header keamanan standar industri.
 * --------------------------------------------------------- */

use axum::{
    body::Body,
    http::{Request, header},
    middleware::Next,
    response::Response,
};

pub async fn security_headers_middleware(
    req: Request<Body>,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;
    
    let headers = response.headers_mut();
    
    // 1. Mencegah Clickjacking
    headers.insert(header::X_FRAME_OPTIONS, "DENY".parse().unwrap());
    
    // 2. Mencegah MIME Sniffing
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    
    // 3. XSS Protection (untuk browser lama)
    headers.insert(header::X_XSS_PROTECTION, "1; mode=block".parse().unwrap());
    
    // 4. Content Security Policy (Lengkap)
    let cfg = crate::Config::load();
    let csp = if cfg.app_debug {
        let port = cfg.vite_port;
        let host = &cfg.app_host;
        let extra_hosts = if host != "0.0.0.0" && host != "127.0.0.1" && host != "localhost" && !host.is_empty() {
            format!("http://{}:{} ws://{}:{} ", host, port, host, port)
        } else {
            "".to_string()
        };

        format!(
            "default-src 'self'; \
             script-src 'self' 'unsafe-inline' 'unsafe-eval' http://localhost:{} http://127.0.0.1:{} {}https:; \
             style-src 'self' 'unsafe-inline' http://localhost:{} http://127.0.0.1:{} {}https:; \
             font-src 'self' https: data:; \
             img-src 'self' data: https:; \
             connect-src 'self' ws://localhost:{} ws://127.0.0.1:{} http://localhost:{} http://127.0.0.1:{} {}https:;",
            port, port, extra_hosts,
            port, port, extra_hosts,
            port, port, port, port, extra_hosts
        )
    } else {
        "default-src 'self'; \
         script-src 'self' 'unsafe-inline' 'unsafe-eval' https:; \
         style-src 'self' 'unsafe-inline' https:; \
         font-src 'self' https: data:; \
         img-src 'self' data: https:; \
         connect-src 'self' https:;".to_string()
    };
    headers.insert(header::CONTENT_SECURITY_POLICY, csp.parse().unwrap());
    
    response
}
