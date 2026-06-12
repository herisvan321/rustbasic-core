pub mod error;

#[cfg(feature = "mysql")]
pub mod mysql;

#[cfg(feature = "sqlite")]
pub mod sqlite;

pub use error::SqlError;

// Unified data types
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    Null,
    Text(String),
    Blob(Vec<u8>),
    Integer(i64),
    Real(f64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlColumn {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SqlRow {
    pub columns: Vec<SqlColumn>,
    pub values: Vec<SqlValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryResult {
    pub rows_affected: u64,
    pub last_insert_id: u64,
}

// ==========================================
// 1. Unified Connection Trait
// ==========================================
pub trait SqlConnection: Send {
    fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Result<QueryResult, SqlError>;
    fn query(&mut self, sql: &str, params: &[SqlValue]) -> Result<Vec<SqlRow>, SqlError>;
}

// ==========================================
// 2. ToSql Trait and Implementations
// ==========================================
pub trait ToSql {
    fn to_sql(&self) -> SqlValue;
}

impl ToSql for String {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Text(self.clone())
    }
}

impl ToSql for &str {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Text(self.to_string())
    }
}

impl ToSql for i64 {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Integer(*self)
    }
}

impl ToSql for i32 {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Integer(*self as i64)
    }
}

impl ToSql for i16 {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Integer(*self as i64)
    }
}

impl ToSql for i8 {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Integer(*self as i64)
    }
}

impl ToSql for u64 {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Integer(*self as i64)
    }
}

impl ToSql for u32 {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Integer(*self as i64)
    }
}

impl ToSql for u16 {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Integer(*self as i64)
    }
}

impl ToSql for u8 {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Integer(*self as i64)
    }
}

impl ToSql for f64 {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Real(*self)
    }
}

impl ToSql for f32 {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Real(*self as f64)
    }
}

impl ToSql for bool {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Integer(if *self { 1 } else { 0 })
    }
}

impl ToSql for Vec<u8> {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Blob(self.clone())
    }
}

impl ToSql for &[u8] {
    fn to_sql(&self) -> SqlValue {
        SqlValue::Blob(self.to_vec())
    }
}

impl<T: ToSql> ToSql for Option<T> {
    fn to_sql(&self) -> SqlValue {
        match self {
            Some(v) => v.to_sql(),
            None => SqlValue::Null,
        }
    }
}

// Helper macro for binding parameters conveniently
#[macro_export]
macro_rules! sql_params {
    ($($val:expr),* $(,)?) => {
        vec![
            $(
                $crate::sql::driver::ToSql::to_sql(&$val)
            ),*
        ]
    };
}

// ==========================================
// 3. Unified URL connection builder
// ==========================================
#[cfg(feature = "mysql")]
struct MysqlUrl {
    host: String,
    port: u16,
    user: String,
    password: String,
    database: String,
}

#[cfg(feature = "mysql")]
fn parse_mysql_url(url: &str) -> Result<MysqlUrl, SqlError> {
    if !url.starts_with("mysql://") {
        return Err(SqlError::Other("Invalid MySQL URL scheme".into()));
    }
    let s = &url["mysql://".len()..];
    
    let (creds, host_db) = if let Some(idx) = s.find('@') {
        (&s[..idx], &s[idx + 1..])
    } else {
        ("", s)
    };
    
    let mut user = String::new();
    let mut password = String::new();
    if !creds.is_empty() {
        if let Some(colon_idx) = creds.find(':') {
            user = creds[..colon_idx].to_string();
            password = creds[colon_idx + 1..].to_string();
        } else {
            user = creds.to_string();
        }
    }
    
    let (host_port, database) = if let Some(slash_idx) = host_db.find('/') {
        (&host_db[..slash_idx], host_db[slash_idx + 1..].to_string())
    } else {
        (host_db, String::new())
    };
    
    let mut host = "127.0.0.1".to_string();
    let mut port = 3306;
    if !host_port.is_empty() {
        if let Some(colon_idx) = host_port.find(':') {
            host = host_port[..colon_idx].to_string();
            if let Ok(p) = host_port[colon_idx + 1..].parse::<u16>() {
                port = p;
            }
        } else {
            host = host_port.to_string();
        }
    }
    
    Ok(MysqlUrl {
        host,
        port,
        user,
        password,
        database,
    })
}

pub fn connect(url: &str) -> Result<Box<dyn SqlConnection>, SqlError> {
    if url.starts_with("sqlite://") {
        let path = &url["sqlite://".len()..];
        #[cfg(feature = "sqlite")]
        {
            let conn = sqlite::SqliteConnection::connect(path)?;
            Ok(Box::new(conn))
        }
        #[cfg(not(feature = "sqlite"))]
        {
            let _ = path;
            Err(SqlError::Other("SQLite feature not enabled".into()))
        }
    } else if url.starts_with("mysql://") {
        #[cfg(feature = "mysql")]
        {
            let parsed = parse_mysql_url(url)?;
            let conn = mysql::MySqlConnection::connect(
                &parsed.host,
                parsed.port,
                &parsed.user,
                &parsed.password,
                &parsed.database,
            )?;
            Ok(Box::new(conn))
        }
        #[cfg(not(feature = "mysql"))]
        {
            Err(SqlError::Other("MySQL feature not enabled".into()))
        }
    } else {
        Err(SqlError::Other(format!("Unsupported database URL scheme: {}", url)))
    }
}

pub trait RowIndex {
    fn index(&self, row: &SqlRow) -> Result<usize, SqlError>;
}

impl RowIndex for usize {
    fn index(&self, row: &SqlRow) -> Result<usize, SqlError> {
        if *self < row.len() {
            Ok(*self)
        } else {
            Err(SqlError::ColumnIndexOutOfBounds {
                len: row.len(),
                index: *self,
            })
        }
    }
}

impl RowIndex for &str {
    fn index(&self, row: &SqlRow) -> Result<usize, SqlError> {
        row.columns
            .iter()
            .position(|col| col.name == *self)
            .ok_or_else(|| SqlError::ColumnNotFound((*self).to_string()))
    }
}

impl RowIndex for String {
    fn index(&self, row: &SqlRow) -> Result<usize, SqlError> {
        row.columns
            .iter()
            .position(|col| col.name == *self)
            .ok_or_else(|| SqlError::ColumnNotFound((*self).to_string()))
    }
}

pub trait FromSql: Sized {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError>;
}

impl<T: FromSql> FromSql for Option<T> {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Null => Ok(None),
            other => T::from_sql(other).map(Some),
        }
    }
}

impl FromSql for String {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Text(s) => Ok(s.clone()),
            SqlValue::Integer(i) => Ok(i.to_string()),
            SqlValue::Real(f) => Ok(f.to_string()),
            SqlValue::Blob(b) => String::from_utf8(b.clone())
                .map_err(|e| SqlError::Decode(format!("Invalid UTF-8 in blob: {}", e))),
            SqlValue::Null => Err(SqlError::Decode("Cannot decode NULL to String".into())),
        }
    }
}

impl FromSql for i64 {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Integer(i) => Ok(*i),
            SqlValue::Text(s) => s.parse::<i64>()
                .map_err(|e| SqlError::Decode(format!("Failed to parse i64: {}", e))),
            SqlValue::Real(f) => Ok(*f as i64),
            _ => Err(SqlError::Decode("Cannot decode to i64".into())),
        }
    }
}

impl FromSql for i32 {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Integer(i) => Ok(*i as i32),
            SqlValue::Text(s) => s.parse::<i32>()
                .map_err(|e| SqlError::Decode(format!("Failed to parse i32: {}", e))),
            SqlValue::Real(f) => Ok(*f as i32),
            _ => Err(SqlError::Decode("Cannot decode to i32".into())),
        }
    }
}

impl FromSql for i16 {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Integer(i) => Ok(*i as i16),
            SqlValue::Text(s) => s.parse::<i16>()
                .map_err(|e| SqlError::Decode(format!("Failed to parse i16: {}", e))),
            SqlValue::Real(f) => Ok(*f as i16),
            _ => Err(SqlError::Decode("Cannot decode to i16".into())),
        }
    }
}

impl FromSql for i8 {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Integer(i) => Ok(*i as i8),
            SqlValue::Text(s) => s.parse::<i8>()
                .map_err(|e| SqlError::Decode(format!("Failed to parse i8: {}", e))),
            SqlValue::Real(f) => Ok(*f as i8),
            _ => Err(SqlError::Decode("Cannot decode to i8".into())),
        }
    }
}

impl FromSql for u64 {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Integer(i) => Ok(*i as u64),
            SqlValue::Text(s) => s.parse::<u64>()
                .map_err(|e| SqlError::Decode(format!("Failed to parse u64: {}", e))),
            SqlValue::Real(f) => Ok(*f as u64),
            _ => Err(SqlError::Decode("Cannot decode to u64".into())),
        }
    }
}

impl FromSql for u32 {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Integer(i) => Ok(*i as u32),
            SqlValue::Text(s) => s.parse::<u32>()
                .map_err(|e| SqlError::Decode(format!("Failed to parse u32: {}", e))),
            SqlValue::Real(f) => Ok(*f as u32),
            _ => Err(SqlError::Decode("Cannot decode to u32".into())),
        }
    }
}

impl FromSql for u16 {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Integer(i) => Ok(*i as u16),
            SqlValue::Text(s) => s.parse::<u16>()
                .map_err(|e| SqlError::Decode(format!("Failed to parse u16: {}", e))),
            SqlValue::Real(f) => Ok(*f as u16),
            _ => Err(SqlError::Decode("Cannot decode to u16".into())),
        }
    }
}

impl FromSql for u8 {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Integer(i) => Ok(*i as u8),
            SqlValue::Text(s) => s.parse::<u8>()
                .map_err(|e| SqlError::Decode(format!("Failed to parse u8: {}", e))),
            SqlValue::Real(f) => Ok(*f as u8),
            _ => Err(SqlError::Decode("Cannot decode to u8".into())),
        }
    }
}

impl FromSql for f64 {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Real(f) => Ok(*f),
            SqlValue::Integer(i) => Ok(*i as f64),
            SqlValue::Text(s) => s.parse::<f64>()
                .map_err(|e| SqlError::Decode(format!("Failed to parse f64: {}", e))),
            _ => Err(SqlError::Decode("Cannot decode to f64".into())),
        }
    }
}

impl FromSql for f32 {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Real(f) => Ok(*f as f32),
            SqlValue::Integer(i) => Ok(*i as f32),
            SqlValue::Text(s) => s.parse::<f32>()
                .map_err(|e| SqlError::Decode(format!("Failed to parse f32: {}", e))),
            _ => Err(SqlError::Decode("Cannot decode to f32".into())),
        }
    }
}

impl FromSql for bool {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Integer(i) => Ok(*i != 0),
            SqlValue::Text(s) => {
                let s_lower = s.to_lowercase();
                if s_lower == "true" || s_lower == "1" || s_lower == "t" || s_lower == "y" || s_lower == "yes" {
                    Ok(true)
                } else if s_lower == "false" || s_lower == "0" || s_lower == "f" || s_lower == "n" || s_lower == "no" || s_lower.is_empty() {
                    Ok(false)
                } else {
                    Err(SqlError::Decode(format!("Cannot decode '{}' to bool", s)))
                }
            }
            _ => Err(SqlError::Decode("Cannot decode to bool".into())),
        }
    }
}

impl FromSql for Vec<u8> {
    fn from_sql(value: &SqlValue) -> Result<Self, SqlError> {
        match value {
            SqlValue::Blob(b) => Ok(b.clone()),
            SqlValue::Text(s) => Ok(s.as_bytes().to_vec()),
            _ => Err(SqlError::Decode("Cannot decode to Vec<u8>".into())),
        }
    }
}

impl SqlRow {
    pub fn len(&self) -> usize {
        self.values.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    
    pub fn column(&self, index: usize) -> &SqlColumn {
        &self.columns[index]
    }
    
    pub fn get_value(&self, name: &str) -> Option<&SqlValue> {
        self.columns.iter().position(|col| col.name == name)
            .map(|idx| &self.values[idx])
    }

    pub fn try_get<T, I>(&self, index: I) -> Result<T, SqlError>
    where
        T: FromSql,
        I: RowIndex,
    {
        let idx = index.index(self)?;
        let val = &self.values[idx];
        T::from_sql(val)
    }

    pub fn get<T, I>(&self, index: I) -> T
    where
        T: FromSql,
        I: RowIndex,
    {
        self.try_get(index).unwrap()
    }
}
