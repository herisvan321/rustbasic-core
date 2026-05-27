#[macro_export]
macro_rules! log_info {
    ($($arg:tt)+) => {
        $crate::logger::log($crate::logger::Level::Info, &format!($($arg)+));
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)+) => {
        $crate::logger::log($crate::logger::Level::Error, &format!($($arg)+));
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)+) => {
        $crate::logger::log($crate::logger::Level::Warn, &format!($($arg)+));
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)+) => {
        $crate::logger::log($crate::logger::Level::Debug, &format!($($arg)+));
    };
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)+) => {
        $crate::logger::log($crate::logger::Level::Trace, &format!($($arg)+));
    };
}

// Re-export the macros under aliases that match the tracing crate's interface
pub use crate::{
    log_debug as debug, log_error as error, log_info as info, log_trace as trace, log_warn as warn,
};
