use crate::requests::Request;
use crate::middleware::Next;
use crate::router::Response;
use crate::colored::Colorize;

pub async fn logging_middleware(
    req: Request,
    next: Next,
) -> Response {
    let method = req.method.clone();
    let path = req.path.clone();
    let ip = req.ip_address.clone();

    // Log ke Terminal (Format: [HTTP] TIMESTAMP METHOD PATH from IP)
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

    // Log ke File (Tanpa warna via custom logger)
    crate::logger::log(crate::logger::Level::Info, &format!("Request method={} path={} ip={}", method, path, ip));

    next.run(req).await
}
