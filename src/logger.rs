/* ---------------------------------------------------------
 * 📑 LABEL: LOGGER & BANNER (config/logger.rs)
 * Pengaturan logging (Terminal + File) dan tampilan visual startup.
 * --------------------------------------------------------- */

use tracing_subscriber::{fmt, EnvFilter, prelude::*, registry};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields};
use tracing_subscriber::fmt::FmtContext;
use tracing::{Event, Subscriber, Level};
use std::fmt as std_fmt;
use colored::*;

struct CustomFormatter;

impl<S, N> FormatEvent<S, N> for CustomFormatter
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: fmt::format::Writer<'_>,
        event: &Event<'_>,
    ) -> std_fmt::Result {
        let metadata = event.metadata();
        let level = metadata.level();
        let timestamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ");

        // Format: [LEVEL] TIMESTAMP MESSAGE
        let level_str = format!("[{}]", level);
        let level_colored = match *level {
            Level::ERROR => level_str.red().bold(),
            Level::WARN => level_str.yellow().bold(),
            Level::INFO => level_str.green().bold(),
            Level::DEBUG => level_str.blue().bold(),
            Level::TRACE => level_str.magenta().bold(),
        };

        write!(writer, "{} {} ", level_colored, timestamp.to_string().dimmed())?;
        
        _ctx.format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

pub fn init() -> WorkerGuard {
    let file_appender = tracing_appender::rolling::daily("storage/logs", "rustbasic.log");
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| 
        "rustbasic=debug,tower_http=debug,axum_session=warn,sqlx=warn".into()
    );

    registry()
        .with(env_filter)
        .with(fmt::layer().event_format(CustomFormatter).with_ansi(true))
        .with(fmt::layer().with_target(false).with_ansi(false).with_writer(non_blocking_file))
        .init();

    print_banner();
    guard
}

fn print_banner() {
    println!(r#"
    
    ██████╗ ██╗   ██╗███████╗████████╗██████╗  █████╗ ███████╗██╗ ██████╗
    ██╔══██╗██║   ██║██╔════╝╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██║██╔════╝
    ██████╔╝██║   ██║███████╗   ██║   ██████╔╝███████║███████╗██║██║     
    ██╔══██╗██║   ██║╚════██║   ██║   ██╔══██╗██╔══██║╚════██║██║██║     
    ██║  ██║╚██████╔╝███████║   ██║   ██████╔╝██║  ██║███████║██║╚██████╗
    ╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   ╚═════╝ ╚═╝  ╚═╝╚══════╝╚═╝ ╚═════╝
    "#);
    println!("    >> RustBasic Full-stack Framework - Version 2026 <<\n");
}
