use std::sync::Arc;
use serde_json::Value;
use super::error::Error;
use super::row::{AnyColumn, AnyTypeInfo, DbValue};
pub use super::row::AnyRow;

pub trait Database {}
impl Database for Any {}

pub struct Any;

pub struct AnyArguments<'q> {
    pub _marker: std::marker::PhantomData<&'q ()>,
}

pub fn install_default_drivers() {}

#[derive(Clone)]
pub enum AnyPool {
    #[cfg(feature = "sqlite")]
    Sqlite(Arc<SqlitePoolInner>),
    #[cfg(feature = "mysql")]
    MySql(crate::sql::driver::mysql::MySqlPool),
}

impl std::fmt::Debug for AnyPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "sqlite")]
            AnyPool::Sqlite(inner) => f.debug_struct("SqlitePool")
                .field("path", &inner.path)
                .finish(),
            #[cfg(feature = "mysql")]
            AnyPool::MySql(_) => write!(f, "MySqlPool"),
        }
    }
}

#[cfg(feature = "sqlite")]
pub struct SqlitePoolInner {
    pub path: String,
    pub connections: tokio::sync::Mutex<Vec<crate::sql::driver::sqlite::SqliteConnection>>,
}

pub struct SqliteConnection {
    #[cfg(feature = "sqlite")]
    pub conn: Option<crate::sql::driver::sqlite::SqliteConnection>,
    #[cfg(feature = "sqlite")]
    pub pool: Option<Arc<SqlitePoolInner>>,
}

impl std::fmt::Debug for SqliteConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SqliteConnection")
    }
}

impl Drop for SqliteConnection {
    fn drop(&mut self) {
        #[cfg(feature = "sqlite")]
        if let (Some(conn), Some(pool)) = (self.conn.take(), &self.pool) {
            let pool = pool.clone();
            tokio::spawn(async move {
                let mut conns = pool.connections.lock().await;
                conns.push(conn);
            });
        }
    }
}

pub enum AnyConnection {
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteConnection),
    #[cfg(feature = "mysql")]
    MySql(Option<crate::sql::driver::mysql::PoolConnection>),
}

impl std::fmt::Debug for AnyConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "sqlite")]
            AnyConnection::Sqlite(_) => write!(f, "AnyConnection::Sqlite"),
            #[cfg(feature = "mysql")]
            AnyConnection::MySql(_) => write!(f, "AnyConnection::MySql"),
        }
    }
}

pub struct AnyQueryResult {
    pub rows_affected: u64,
    pub last_insert_id: Option<i64>,
}

impl AnyQueryResult {
    pub fn rows_affected(&self) -> u64 {
        self.rows_affected
    }

    pub fn last_insert_id(&self) -> Option<i64> {
        self.last_insert_id
    }
}

#[cfg(feature = "mysql")]
#[derive(Clone)]
pub struct MySqlPool(pub crate::sql::driver::mysql::MySqlPool);

#[cfg(feature = "mysql")]
impl MySqlPool {
    pub async fn connect(url: &str) -> Result<Self, Error> {
        let parsed = parse_mysql_url(url)?;
        let pool = crate::sql::driver::mysql::MySqlPool::new(
            &parsed.host,
            parsed.port,
            &parsed.user,
            &parsed.password,
            &parsed.database,
        );
        {
            let mut conn = pool.acquire().map_err(|e| Error::Database(e.to_string()))?;
            let _ping = conn.execute("SELECT 1", &[]).map_err(|e| Error::Database(e.to_string()))?;
        }
        Ok(MySqlPool(pool))
    }
}

impl AnyPool {
    pub async fn connect(url: &str) -> Result<Self, Error> {
        if url.starts_with("sqlite:") {
            #[cfg(feature = "sqlite")]
            {
                let path = url.trim_start_matches("sqlite:")
                    .split('?')
                    .next()
                    .unwrap_or(url);
                if let Some(parent) = std::path::Path::new(path).parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let conn = crate::sql::driver::sqlite::SqliteConnection::connect(path)
                    .map_err(|e| Error::Database(e.to_string()))?;

                let inner = Arc::new(SqlitePoolInner {
                    path: path.to_string(),
                    connections: tokio::sync::Mutex::new(vec![conn]),
                });
                Ok(AnyPool::Sqlite(inner))
            }
            #[cfg(not(feature = "sqlite"))]
            {
                let _ = url;
                Err(Error::Database("sqlite feature not enabled".to_string()))
            }
        } else if url.starts_with("mysql://") {
            #[cfg(feature = "mysql")]
            {
                let parsed = parse_mysql_url(url)?;
                let pool = crate::sql::driver::mysql::MySqlPool::new(
                    &parsed.host,
                    parsed.port,
                    &parsed.user,
                    &parsed.password,
                    &parsed.database,
                );
                {
                    let mut conn = pool.acquire().map_err(|e| Error::Database(e.to_string()))?;
                    let _ping = conn.execute("SELECT 1", &[]).map_err(|e| Error::Database(e.to_string()))?;
                }
                Ok(AnyPool::MySql(pool))
            }
            #[cfg(not(feature = "mysql"))]
            {
                let _ = url;
                Err(Error::Database(
                    "DB_CONNECTION=mysql terdeteksi, tapi fitur mysql belum aktif.\n\
                     Tambahkan features = [\"mysql\"] pada rustbasic-core di Cargo.toml project Anda".to_string()
                ))
            }
        } else {
            Err(Error::Database(format!("Unsupported database URL prefix: {}", url)))
        }
    }

    pub async fn acquire(&self) -> Result<PoolConnection, Error> {
        let conn = match self {
            #[cfg(feature = "sqlite")]
            AnyPool::Sqlite(pool) => {
                let mut conns = pool.connections.lock().await;
                let conn = if let Some(c) = conns.pop() {
                    c
                } else {
                    let path = &pool.path;
                    let c = crate::sql::driver::sqlite::SqliteConnection::connect(path)
                        .map_err(|e| Error::Database(e.to_string()))?;
                    c
                };
                AnyConnection::Sqlite(SqliteConnection {
                    conn: Some(conn),
                    pool: Some(pool.clone()),
                })
            }
            #[cfg(feature = "mysql")]
            AnyPool::MySql(pool) => {
                let conn = pool.acquire()
                    .map_err(|e| Error::Database(e.to_string()))?;
                AnyConnection::MySql(Some(conn))
            }
        };
        Ok(PoolConnection { conn })
    }

    pub fn backend_name(&self) -> &str {
        match self {
            #[cfg(feature = "sqlite")]
            AnyPool::Sqlite(_) => "SQLite",
            #[cfg(feature = "mysql")]
            AnyPool::MySql(_) => "MySQL",
        }
    }
}

impl AnyConnection {
    pub fn backend_name(&self) -> &str {
        match self {
            #[cfg(feature = "sqlite")]
            AnyConnection::Sqlite(_) => "SQLite",
            #[cfg(feature = "mysql")]
            AnyConnection::MySql(_) => "MySQL",
        }
    }
}

#[derive(Debug)]
pub struct PoolConnection {
    pub conn: AnyConnection,
}

impl std::ops::Deref for PoolConnection {
    type Target = AnyConnection;
    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

impl std::ops::DerefMut for PoolConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}

#[allow(async_fn_in_trait)]
pub trait Executor {
    type Database: Database;

    async fn execute(self, sql: &str, arguments: &[Value]) -> Result<AnyQueryResult, Error>;
    async fn fetch_all(self, sql: &str, arguments: &[Value]) -> Result<Vec<AnyRow>, Error>;
    async fn fetch_optional(self, sql: &str, arguments: &[Value]) -> Result<Option<AnyRow>, Error>;
    async fn fetch_one(self, sql: &str, arguments: &[Value]) -> Result<AnyRow, Error>;
}

impl<'c> Executor for &'c AnyPool {
    type Database = Any;

    async fn execute(self, sql: &str, arguments: &[Value]) -> Result<AnyQueryResult, Error> {
        let mut conn = self.acquire().await?;
        conn.execute_internal(sql, arguments).await
    }

    async fn fetch_all(self, sql: &str, arguments: &[Value]) -> Result<Vec<AnyRow>, Error> {
        let mut conn = self.acquire().await?;
        conn.fetch_all_internal(sql, arguments).await
    }

    async fn fetch_optional(self, sql: &str, arguments: &[Value]) -> Result<Option<AnyRow>, Error> {
        let mut conn = self.acquire().await?;
        conn.fetch_optional_internal(sql, arguments).await
    }

    async fn fetch_one(self, sql: &str, arguments: &[Value]) -> Result<AnyRow, Error> {
        let mut conn = self.acquire().await?;
        conn.fetch_one_internal(sql, arguments).await
    }
}

impl<'c> Executor for &'c mut AnyConnection {
    type Database = Any;

    async fn execute(self, sql: &str, arguments: &[Value]) -> Result<AnyQueryResult, Error> {
        self.execute_internal(sql, arguments).await
    }

    async fn fetch_all(self, sql: &str, arguments: &[Value]) -> Result<Vec<AnyRow>, Error> {
        self.fetch_all_internal(sql, arguments).await
    }

    async fn fetch_optional(self, sql: &str, arguments: &[Value]) -> Result<Option<AnyRow>, Error> {
        self.fetch_optional_internal(sql, arguments).await
    }

    async fn fetch_one(self, sql: &str, arguments: &[Value]) -> Result<AnyRow, Error> {
        self.fetch_one_internal(sql, arguments).await
    }
}

#[cfg(feature = "mysql")]
impl<'c> Executor for &'c MySqlPool {
    type Database = Any;

    async fn execute(self, sql: &str, arguments: &[Value]) -> Result<AnyQueryResult, Error> {
        let mut conn = self.0.acquire()
            .map_err(|e| Error::Database(e.to_string()))?;
        let sql_args: Vec<crate::sql::driver::SqlValue> = arguments.iter().map(json_to_sql_value).collect();
        let res = conn.execute(sql, &sql_args)
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(AnyQueryResult {
            rows_affected: res.rows_affected,
            last_insert_id: Some(res.last_insert_id as i64),
        })
    }

    async fn fetch_all(self, _sql: &str, _arguments: &[Value]) -> Result<Vec<AnyRow>, Error> {
        Err(Error::Database("Not implemented for MySqlPool".to_string()))
    }

    async fn fetch_optional(self, _sql: &str, _arguments: &[Value]) -> Result<Option<AnyRow>, Error> {
        Err(Error::Database("Not implemented for MySqlPool".to_string()))
    }

    async fn fetch_one(self, _sql: &str, _arguments: &[Value]) -> Result<AnyRow, Error> {
        Err(Error::Database("Not implemented for MySqlPool".to_string()))
    }
}

impl AnyConnection {
    pub async fn execute_internal(&mut self, sql: &str, arguments: &[Value]) -> Result<AnyQueryResult, Error> {
        let sql_args: Vec<crate::sql::driver::SqlValue> = arguments.iter().map(json_to_sql_value).collect();
        match self {
            #[cfg(feature = "sqlite")]
            AnyConnection::Sqlite(s_conn) => {
                let mut conn = s_conn.conn.take().ok_or_else(|| Error::Database("SQLite connection already used or dropped".to_string()))?;
                let sql_str = sql.to_string();
                let (conn_ret, res) = tokio::task::spawn_blocking(move || {
                    let result = conn.execute(&sql_str, &sql_args)
                        .map_err(|e| Error::Database(e.to_string()));
                    (conn, result)
                }).await.map_err(|e| Error::Database(e.to_string()))?;
                s_conn.conn = Some(conn_ret);
                let query_res = res?;
                Ok(AnyQueryResult {
                    rows_affected: query_res.rows_affected,
                    last_insert_id: Some(query_res.last_insert_id as i64),
                })
            }
            #[cfg(feature = "mysql")]
            AnyConnection::MySql(m_conn) => {
                let mut conn = m_conn.take().ok_or_else(|| Error::Database("MySQL connection already used or dropped".to_string()))?;
                let sql_str = sql.to_string();
                let (conn_ret, res) = tokio::task::spawn_blocking(move || {
                    let result = conn.execute(&sql_str, &sql_args)
                        .map_err(|e| Error::Database(e.to_string()));
                    (conn, result)
                }).await.map_err(|e| Error::Database(e.to_string()))?;
                *m_conn = Some(conn_ret);
                let query_res = res?;
                Ok(AnyQueryResult {
                    rows_affected: query_res.rows_affected,
                    last_insert_id: Some(query_res.last_insert_id as i64),
                })
            }
        }
    }

    pub async fn fetch_all_internal(&mut self, sql: &str, arguments: &[Value]) -> Result<Vec<AnyRow>, Error> {
        let sql_args: Vec<crate::sql::driver::SqlValue> = arguments.iter().map(json_to_sql_value).collect();
        match self {
            #[cfg(feature = "sqlite")]
            AnyConnection::Sqlite(s_conn) => {
                let mut conn = s_conn.conn.take().ok_or_else(|| Error::Database("SQLite connection already used or dropped".to_string()))?;
                let sql_str = sql.to_string();
                let (conn_ret, res) = tokio::task::spawn_blocking(move || {
                    let result = conn.query(&sql_str, &sql_args)
                        .map_err(|e| Error::Database(e.to_string()));
                    (conn, result)
                }).await.map_err(|e| Error::Database(e.to_string()))?;
                s_conn.conn = Some(conn_ret);
                let rows = res?;
                let any_rows = rows.into_iter().map(sql_row_to_any_row).collect();
                Ok(any_rows)
            }
            #[cfg(feature = "mysql")]
            AnyConnection::MySql(m_conn) => {
                let mut conn = m_conn.take().ok_or_else(|| Error::Database("MySQL connection already used or dropped".to_string()))?;
                let sql_str = sql.to_string();
                let (conn_ret, res) = tokio::task::spawn_blocking(move || {
                    let result = conn.query(&sql_str, &sql_args)
                        .map_err(|e| Error::Database(e.to_string()));
                    (conn, result)
                }).await.map_err(|e| Error::Database(e.to_string()))?;
                *m_conn = Some(conn_ret);
                let rows = res?;
                let any_rows = rows.into_iter().map(sql_row_to_any_row).collect();
                Ok(any_rows)
            }
        }
    }

    pub async fn fetch_optional_internal(&mut self, sql: &str, arguments: &[Value]) -> Result<Option<AnyRow>, Error> {
        let mut rows = self.fetch_all_internal(sql, arguments).await?;
        if rows.is_empty() {
            Ok(None)
        } else {
            Ok(Some(rows.remove(0)))
        }
    }

    pub async fn fetch_one_internal(&mut self, sql: &str, arguments: &[Value]) -> Result<AnyRow, Error> {
        let mut rows = self.fetch_all_internal(sql, arguments).await?;
        if rows.is_empty() {
            Err(Error::RowNotFound)
        } else {
            Ok(rows.remove(0))
        }
    }
}

fn json_to_sql_value(val: &Value) -> crate::sql::driver::SqlValue {
    match val {
        Value::Null => crate::sql::driver::SqlValue::Null,
        Value::Bool(b) => crate::sql::driver::SqlValue::Integer(if *b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                crate::sql::driver::SqlValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                crate::sql::driver::SqlValue::Real(f)
            } else {
                crate::sql::driver::SqlValue::Real(0.0)
            }
        }
        Value::String(s) => crate::sql::driver::SqlValue::Text(s.clone()),
        Value::Array(arr) => {
            let bytes: Vec<u8> = arr.iter().filter_map(|v| v.as_u64().map(|b| b as u8)).collect();
            crate::sql::driver::SqlValue::Blob(bytes)
        }
        _ => crate::sql::driver::SqlValue::Text(val.to_string()),
    }
}

fn sql_row_to_any_row(row: crate::sql::driver::SqlRow) -> AnyRow {
    let mut columns = Vec::new();
    let mut values = Vec::new();

    for col in &row.columns {
        columns.push(AnyColumn {
            name: col.name.clone(),
            type_info: AnyTypeInfo {
                name: "UNKNOWN".to_string(),
            },
        });
    }

    for val in &row.values {
        let db_val = match val {
            crate::sql::driver::SqlValue::Null => DbValue::Null,
            crate::sql::driver::SqlValue::Text(s) => DbValue::Text(s.clone()),
            crate::sql::driver::SqlValue::Blob(b) => DbValue::Blob(b.clone()),
            crate::sql::driver::SqlValue::Integer(i) => DbValue::Integer(*i),
            crate::sql::driver::SqlValue::Real(f) => DbValue::Real(*f),
        };
        values.push(db_val);
    }

    AnyRow { columns, values }
}

#[cfg(feature = "mysql")]
struct MysqlUrl {
    host: String,
    port: u16,
    user: String,
    password: String,
    database: String,
}

#[cfg(feature = "mysql")]
fn parse_mysql_url(url: &str) -> Result<MysqlUrl, Error> {
    if !url.starts_with("mysql://") {
        return Err(Error::Database("Invalid MySQL URL scheme".into()));
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
