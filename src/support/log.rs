use crate::logger::{log, Level};

pub struct Log;

impl Log {
    /// Menulis pesan info ke log (Log::info())
    pub fn info(msg: impl AsRef<str>) {
        log(Level::Info, msg.as_ref());
    }

    /// Menulis pesan debug ke log (Log::debug())
    pub fn debug(msg: impl AsRef<str>) {
        log(Level::Debug, msg.as_ref());
    }

    /// Menulis pesan peringatan ke log (Log::warning())
    pub fn warning(msg: impl AsRef<str>) {
        log(Level::Warn, msg.as_ref());
    }

    /// Menulis pesan error ke log (Log::error())
    pub fn error(msg: impl AsRef<str>) {
        log(Level::Error, msg.as_ref());
    }

    /// Mencatat pesan info dengan konteks JSON (Log::info($msg, $context))
    pub fn info_with(msg: impl AsRef<str>, context: &serde_json::Value) {
        let msg_with_ctx = format!("{} - Context: {}", msg.as_ref(), context);
        log(Level::Info, &msg_with_ctx);
    }

    /// Mencatat pesan debug dengan konteks JSON (Log::debug($msg, $context))
    pub fn debug_with(msg: impl AsRef<str>, context: &serde_json::Value) {
        let msg_with_ctx = format!("{} - Context: {}", msg.as_ref(), context);
        log(Level::Debug, &msg_with_ctx);
    }

    /// Mencatat pesan peringatan dengan konteks JSON (Log::warning($msg, $context))
    pub fn warning_with(msg: impl AsRef<str>, context: &serde_json::Value) {
        let msg_with_ctx = format!("{} - Context: {}", msg.as_ref(), context);
        log(Level::Warn, &msg_with_ctx);
    }

    /// Mencatat pesan error dengan konteks JSON (Log::error($msg, $context))
    pub fn error_with(msg: impl AsRef<str>, context: &serde_json::Value) {
        let msg_with_ctx = format!("{} - Context: {}", msg.as_ref(), context);
        log(Level::Error, &msg_with_ctx);
    }
}
