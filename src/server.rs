use axum::{Router, response::IntoResponse, ServiceExt, handler::HandlerWithoutStateExt};
use tower_http::services::ServeDir;
use tower_http::normalize_path::NormalizePathLayer;
use tower::Layer;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer, key_extractor::SmartIpKeyExtractor};
use axum_session::{SessionLayer, SessionStore};
use crate::app::Config;
use crate::session_manager::RustBasicSessionStore;
use crate::errors::ErrorController;
use tower_governor::GovernorError;
use std::net::SocketAddr;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use std::process::Command;
use std::time::Duration;
use tower_livereload::LiveReloadLayer;

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Arc<Config>,
}

pub async fn start_server(
    cfg: Config, 
    session_store: SessionStore<RustBasicSessionStore>,
    static_files: ServeDir,
    db: DatabaseConnection,
    app_router: Router<AppState>
) {
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

    // 1.5 Konfigurasi Rate Limiting
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .period(Duration::from_millis(1000 / cfg.app_limit_request))
            .burst_size(cfg.app_limit_request as u32)
            .finish()
            .unwrap(),
    );

    // 2. Bangun Router
    let app = Router::new()
        .merge(app_router)
        .fallback_service(static_files.not_found_service(ErrorController::not_found.into_service()))
        .layer(axum::middleware::from_fn(crate::middleware::security_headers::security_headers_middleware))
        .layer(axum::middleware::from_fn(crate::middleware::logging::logging_middleware))
        .layer(GovernorLayer::new(governor_conf))
        .layer(SessionLayer::new(session_store))
        .with_state(state);

    // 2.5 Live Reload (Hanya aktif jika APP_DEBUG=true)
    let app = if cfg.app_debug {
        tracing::info!("🔄 Fitur Live Reload (Auto-refresh) diaktifkan.");
        app.layer(LiveReloadLayer::new())
    } else {
        app
    };
    
    // 2.6 Normalisasi Path (Menangani trailing slash /home/ -> /home)
    let app = NormalizePathLayer::trim_trailing_slash().layer(app);

    // 3. Tentukan Alamat
    let addr_str = format!("{}:{}", cfg.app_host, cfg.app_port);
    let addr: SocketAddr = addr_str.parse().expect("Alamat server tidak valid");
    
    tracing::info!("{} berjalan di: http://{}", cfg.app_name, addr);
    
    // 4. Jalankan Server dengan ConnectInfo agar IP bisa dideteksi
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, ServiceExt::<axum::extract::Request>::into_make_service_with_connect_info::<SocketAddr>(app)).await.unwrap();
}

/// Membunuh proses yang menggunakan port tertentu agar tidak terjadi error "Address already in use"
fn kill_port_if_in_use(port: u16) {
    #[cfg(target_os = "macos")]
    {
        // Mencari PID yang menggunakan port tersebut
        let output = Command::new("lsof")
            .arg("-t")
            .arg(format!("-i:{}", port))
            .output();

        if let Ok(out) = output {
            let pid_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !pid_str.is_empty() {
                tracing::warn!("Port {} sedang digunakan oleh PID {}. Membunuh proses...", port, pid_str);
                
                // Membunuh proses tersebut
                for pid in pid_str.split('\n') {
                    if !pid.is_empty() {
                        let _ = Command::new("kill")
                            .arg("-9")
                            .arg(pid)
                            .output();
                    }
                }

                // Beri waktu sejenak agar OS melepas port (Penting agar tidak panic AddrInUse)
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
}

/// Menangani error dari Rate Limiter (Governor) dengan tampilan HTML Premium
#[allow(dead_code)]
fn handle_governor_error(err: GovernorError) -> axum::response::Response {
    match err {
        GovernorError::TooManyRequests { wait_time, .. } => {
            ErrorController::show(
                429, 
                &format!("Terlalu banyak permintaan. Silakan tunggu {} detik lagi.", wait_time)
            ).into_response()
        },
        _ => ErrorController::show(500, "Terjadi kesalahan pada sistem pembatas request.").into_response(),
    }
}
