use std::fmt;

#[derive(Debug)]
pub enum Error {
    Database(String),
    RowNotFound,
    ColumnNotFound(String),
    ColumnIndexOutOfBounds { len: usize, index: usize },
    DecodeError(String),
    Protocol(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Database(msg) => write!(f, "Database error: {}", msg),
            Error::RowNotFound => write!(f, "Row not found"),
            Error::ColumnNotFound(col) => write!(f, "Column not found: {}", col),
            Error::ColumnIndexOutOfBounds { len, index } => {
                write!(f, "Column index out of bounds: len {}, index {}", len, index)
            }
            Error::DecodeError(msg) => write!(f, "Decode error: {}", msg),
            Error::Protocol(msg) => write!(f, "Protocol error: {}", msg),
        }
    }
}
