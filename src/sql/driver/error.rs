use std::fmt;

#[derive(Debug)]
pub enum SqlError {
    Io(std::io::Error),
    Protocol(String),
    Server {
        code: u16,
        sql_state: String,
        message: String,
    },
    RowNotFound,
    ColumnNotFound(String),
    ColumnIndexOutOfBounds { len: usize, index: usize },
    Decode(String),
    Other(String),
}

impl std::error::Error for SqlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SqlError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for SqlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SqlError::Io(e) => write!(f, "I/O error: {}", e),
            SqlError::Protocol(s) => write!(f, "MySQL protocol error: {}", s),
            SqlError::Server { code, sql_state, message } => {
                write!(f, "MySQL server error (code {}): [{}] {}", code, sql_state, message)
            }
            SqlError::RowNotFound => write!(f, "Row not found"),
            SqlError::ColumnNotFound(name) => write!(f, "Column not found: '{}'", name),
            SqlError::ColumnIndexOutOfBounds { len, index } => {
                write!(f, "Column index out of bounds: length is {}, index is {}", len, index)
            }
            SqlError::Decode(s) => write!(f, "Decode error: {}", s),
            SqlError::Other(s) => write!(f, "Other error: {}", s),
        }
    }
}

impl From<std::io::Error> for SqlError {
    fn from(err: std::io::Error) -> Self {
        SqlError::Io(err)
    }
}
