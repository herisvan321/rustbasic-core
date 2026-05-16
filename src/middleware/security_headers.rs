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
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        concat!(
            "default-src 'self'; ",
            "script-src 'self' 'unsafe-inline' 'unsafe-eval' https:; ",
            "style-src 'self' 'unsafe-inline' https:; ",
            "font-src 'self' https: data:; ",
            "img-src 'self' data: https:; ",
            "connect-src 'self' https:;"
        ).parse().unwrap()
    );
    
    response
}
