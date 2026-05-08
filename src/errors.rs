/* ---------------------------------------------------------
 * 📑 LABEL: ERRORS (config/errors.rs)
 * Menangani berbagai kode error HTTP dengan tampilan premium.
 * --------------------------------------------------------- */

use crate::view::render;
use axum::{
    http::StatusCode,
    response::IntoResponse,
};
use minijinja::context;

pub struct ErrorController;

impl ErrorController {
    /// Handler umum untuk menampilkan halaman error
    pub fn show(code: u16, message: &str) -> impl IntoResponse {
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
        }))
    }

    /// Khusus untuk 404 Not Found (digunakan sebagai fallback)
    pub async fn not_found() -> impl IntoResponse {
        Self::show(404, "Maaf, halaman yang Anda cari tidak ditemukan.")
    }
}
