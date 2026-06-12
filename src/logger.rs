use std::fs::OpenOptions;
use std::io::Write;
use crate::chrono::Local;
use crate::colored::Colorize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        };
        write!(f, "{}", s)
    }
}

pub struct LoggerGuard;

/// Initialize the logger. Displays the banner and returns a dummy guard to keep API compatibility.
pub fn init() -> LoggerGuard {
    print_banner();
    LoggerGuard
}

/// Core logging function that handles console output with colors and appends to a daily rolling log file.
pub fn log(level: Level, msg: &str) {
    let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string();

    let level_str = format!("[{}]", level);
    let level_colored = match level {
        Level::Error => level_str.red().bold(),
        Level::Warn => level_str.yellow().bold(),
        Level::Info => level_str.green().bold(),
        Level::Debug => level_str.blue().bold(),
        Level::Trace => level_str.magenta().bold(),
    };

    let console_line = format!("{} {} {}", level_colored, timestamp.to_string().dimmed(), msg);
    println!("{}", console_line);

    // Ensure logs directory exists
    let _ = std::fs::create_dir_all("storage/logs");

    // Write to daily file (rolling)
    let date_str = Local::now().format("%Y-%m-%d").to_string();
    let log_file_path = format!("storage/logs/rustbasic.log.{}", date_str);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&log_file_path)
    {
        let file_line = format!("{} {} {}\n", timestamp, level_str, msg);
        let _ = file.write_all(file_line.as_bytes());
    }
}

fn print_banner() {
    println!();
    println!("    \x1b[38;5;208m██████╗ ██╗   ██╗███████╗████████╗\x1b[38;5;245m██████╗  █████╗ ███████╗██╗ ██████╗\x1b[0m");
    println!("    \x1b[38;5;208m██╔══██╗██║   ██║██╔════╝╚══██╔══╝\x1b[38;5;245m██╔══██╗██╔══██╗██╔════╝██║██╔════╝\x1b[0m");
    println!("    \x1b[38;5;208m██████╔╝██║   ██║███████╗   ██║   \x1b[38;5;245m██████╔╝███████║███████╗██║██║     \x1b[0m");
    println!("    \x1b[38;5;208m██╔══██╗██║   ██║╚════██║   ██║   \x1b[38;5;245m██╔══██╗██╔══██║╚════██║██║██║     \x1b[0m");
    println!("    \x1b[38;5;208m██║  ██║╚██████╔╝███████║   ██║   \x1b[38;5;245m██████╔╝██║  ██║███████║██║╚██████╗\x1b[0m");
    println!("    \x1b[38;5;208m╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   \x1b[38;5;245m╚═════╝ ╚═╝  ╚═╝╚══════╝╚═╝ ╚═════╝\x1b[0m");
    println!();
    println!("    >> \x1b[1;38;5;208mRust\x1b[0m\x1b[1;38;5;245mBasic\x1b[0m Full-stack Framework - Version 2026 <<");
    println!();
}

