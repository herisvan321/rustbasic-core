use crate::requests::Request;
use crate::middleware::Next;
use crate::router::Response;
use http::header;

pub async fn security_headers_middleware(
    req: Request,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;
    
    let headers = response.headers_mut();
    
    // 1. Mencegah Clickjacking (SAMEORIGIN adalah standar industri untuk fleksibilitas internal)
    headers.insert(header::X_FRAME_OPTIONS, "SAMEORIGIN".parse().unwrap());
    
    // 2. Mencegah MIME Sniffing
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    
    // 3. XSS Protection (untuk kompatibilitas browser lama)
    headers.insert(header::X_XSS_PROTECTION, "1; mode=block".parse().unwrap());
    
    // 4. Referrer Policy (Standar modern untuk keamanan kebocoran URL referrer)
    headers.insert(header::REFERRER_POLICY, "strict-origin-when-cross-origin".parse().unwrap());
    
    // 5. Permissions Policy (Membatasi akses API sensor hardware sensitif)
    headers.insert(
        http::header::HeaderName::from_static("permissions-policy"),
        "camera=(self), microphone=(self), geolocation=(self), payment=()".parse().unwrap()
    );
    
    let cfg = crate::Config::load();
    
    // 6. Strict-Transport-Security (HSTS - Wajib HTTPS di mode Produksi)
    if !cfg.app_debug {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            "max-age=31536000; includeSubDomains; preload".parse().unwrap()
        );
    }
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
             frame-src 'self' https:; \
             media-src 'self' https:; \
             object-src 'self' https:; \
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
         frame-src 'self' https:; \
         media-src 'self' https:; \
         object-src 'self' https:; \
         connect-src 'self' https:;".to_string()
    };
    headers.insert(header::CONTENT_SECURITY_POLICY, csp.parse().unwrap());
    
    response
}
