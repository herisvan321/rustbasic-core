use crate::{Request, Response, Next};
use crate::http::{Method, StatusCode};

pub async fn cors_middleware(req: Request, next: Next) -> Response {
    if req.method == Method::OPTIONS {
        return crate::http::Response::builder()
            .status(StatusCode::OK)
            .header("access-control-allow-origin", "*")
            .header("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS")
            .header("access-control-allow-headers", "*")
            .body(Vec::new())
            .unwrap();
    }

    let mut res: Response = next.run(req).await;
    let headers = res.headers_mut();
    headers.insert("access-control-allow-origin", "*".parse().unwrap());
    headers.insert("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap());
    headers.insert("access-control-allow-headers", "*".parse().unwrap());
    res
}
