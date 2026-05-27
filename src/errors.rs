use crate::view::render;
use crate::router::{IntoResponse, Response};
use minijinja::context;
use http::StatusCode;

pub struct ErrorController;

impl ErrorController {
    /// Handler umum untuk menampilkan halaman error
    pub fn show(code: u16, message: &str) -> Response {
        let status = StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        
        let title = match code {
            401 => "Unauthorized",
            402 => "Payment Required",
            403 => "Forbidden",
            404 => "Page Not Found",
            419 => "Page Expired",
            429 => "Too Many Requests",
            500 => "Server Error",
            503 => "Service Unavailable",
            _ => "Error",
        };

        (status, render("errors/minimal.rb.html", context! {
            code => code,
            title => title,
            message => message
        })).into_response()
    }

    /// Khusus untuk 404 Not Found (digunakan sebagai fallback)
    pub async fn not_found() -> Response {
        Self::show(404, "Maaf, halaman yang Anda cari tidak ditemukan.")
    }
}
