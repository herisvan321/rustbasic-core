use serde::Serialize;
use crate::router::{Response, IntoResponse, Html, Json, Redirect};
use crate::session::Session;

pub struct ResponseHelper;

impl ResponseHelper {
    /// Mengembalikan tampilan HTML (Minijinja)
    #[allow(dead_code)]
    pub fn view(html_content: String) -> Response {
        Html(html_content).into_response()
    }

    /// Mengembalikan data JSON
    pub fn json<T: Serialize>(data: T) -> Response {
        Json(data).into_response()
    }

    /// Melakukan pengalihan (Redirect)
    #[allow(dead_code)]
    pub fn redirect(url: &str) -> Response {
        Redirect::to(url).into_response()
    }

    /// Mengembalikan pesan sukses sederhana
    #[allow(dead_code)]
    pub fn success(message: &str) -> Response {
        Json(serde_json::json!({
            "status": "success",
            "message": message
        })).into_response()
    }

    #[allow(dead_code)]
    pub fn not_found() -> Response {
        Json(serde_json::json!({
            "status": "error",
            "message": "Resource not found"
        })).into_response()
    }

    #[allow(dead_code)]
    pub fn error(message: &str) -> Response {
        Json(serde_json::json!({
            "status": "error",
            "message": message
        })).into_response()
    }

    #[allow(dead_code)]
    pub fn internal_server_error() -> Response {
        Json(serde_json::json!({
            "status": "error",
            "message": "Internal server error"
        })).into_response()
    }

    /// Redirect dengan pesan sukses (Flash Message)
    pub fn redirect_with_success(
        url: &str, 
        message: &str, 
        session: Session
    ) -> Response {
        session.set("flash_success", message);
        Redirect::to(url).into_response()
    }

    /// Redirect dengan pesan error (Flash Message)
    pub fn redirect_with_error(
        url: &str, 
        message: &str, 
        session: Session
    ) -> Response {
        session.set("flash_error", message);
        Redirect::to(url).into_response()
    }
}
