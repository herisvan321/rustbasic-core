use serde_json::Value;
pub use super::error::Error;
use super::row::{AnyColumn, AnyTypeInfo, DbValue};
pub use super::row::AnyRow;
use sqlx::{AnyPool as SqlxAnyPool, Executor as SqlxExecutor, Row, Column, TypeInfo, ValueRef};
use sqlx::any::{AnyConnectOptions, AnyRow as SqlxAnyRow};
use std::str::FromStr;

// Re-export sqlx::Any so it's accessible as sql::Any
pub type Any = sqlx::Any;
pub type SqlxAnyConnection = sqlx::AnyConnection;

pub trait Database {}
impl Database for Any {}

#[cfg(feature = "mysql")]
pub type MySqlPool = AnyPool;

#[cfg(feature = "sqlite")]
pub type SqlitePool = AnyPool;

pub struct AnyArguments<'q> {
    pub _marker: std::marker::PhantomData<&'q ()>,
}

pub fn install_default_drivers() {
    sqlx::any::install_default_drivers();
}

#[derive(Clone)]
pub enum AnyPool {
    Sqlx(SqlxAnyPool),
}

impl std::fmt::Debug for AnyPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnyPool")
    }
}

pub struct AnyConnection {
    pub conn: SqlxAnyConnection,
}

impl std::fmt::Debug for AnyConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnyConnection")
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

impl AnyPool {
    pub async fn connect(url: &str) -> Result<Self, Error> {
        install_default_drivers();
        
        let url = if url.starts_with("sqlite:") {
            let path = url.trim_start_matches("sqlite:")
                .split('?')
                .next()
                .unwrap_or(url);
            if let Some(parent) = std::path::Path::new(path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            url.to_string()
        } else {
            url.to_string()
        };

        let options = AnyConnectOptions::from_str(&url)
            .map_err(|e| Error::Database(e.to_string()))?;
        
        let pool = SqlxAnyPool::connect_with(options)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
            
        Ok(AnyPool::Sqlx(pool))
    }

    pub async fn acquire(&self) -> Result<PoolConnection, Error> {
        match self {
            AnyPool::Sqlx(pool) => {
                let conn = pool.acquire()
                    .await
                    .map_err(|e| Error::Database(e.to_string()))?;
                Ok(PoolConnection { conn: AnyConnection { conn: conn.detach() } })
            }
        }
    }

    pub fn backend_name(&self) -> &str {
        "SQLx"
    }
}

impl AnyConnection {
    pub fn backend_name(&self) -> &str {
        "SQLx"
    }
}

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

impl Executor for &AnyPool {
    type Database = Any;

    async fn execute(self, sql: &str, arguments: &[Value]) -> Result<AnyQueryResult, Error> {
        let AnyPool::Sqlx(pool) = self;
        execute_sqlx(pool, sql, arguments).await
    }

    async fn fetch_all(self, sql: &str, arguments: &[Value]) -> Result<Vec<AnyRow>, Error> {
        let AnyPool::Sqlx(pool) = self;
        fetch_all_sqlx(pool, sql, arguments).await
    }

    async fn fetch_optional(self, sql: &str, arguments: &[Value]) -> Result<Option<AnyRow>, Error> {
        let AnyPool::Sqlx(pool) = self;
        fetch_optional_sqlx(pool, sql, arguments).await
    }

    async fn fetch_one(self, sql: &str, arguments: &[Value]) -> Result<AnyRow, Error> {
        let AnyPool::Sqlx(pool) = self;
        fetch_one_sqlx(pool, sql, arguments).await
    }
}

impl Executor for &mut AnyConnection {
    type Database = Any;

    async fn execute(self, sql: &str, arguments: &[Value]) -> Result<AnyQueryResult, Error> {
        execute_sqlx(&mut self.conn, sql, arguments).await
    }

    async fn fetch_all(self, sql: &str, arguments: &[Value]) -> Result<Vec<AnyRow>, Error> {
        fetch_all_sqlx(&mut self.conn, sql, arguments).await
    }

    async fn fetch_optional(self, sql: &str, arguments: &[Value]) -> Result<Option<AnyRow>, Error> {
        fetch_optional_sqlx(&mut self.conn, sql, arguments).await
    }

    async fn fetch_one(self, sql: &str, arguments: &[Value]) -> Result<AnyRow, Error> {
        fetch_one_sqlx(&mut self.conn, sql, arguments).await
    }
}

async fn execute_sqlx<'e, E>(executor: E, sql: &str, arguments: &[Value]) -> Result<AnyQueryResult, Error> 
where E: SqlxExecutor<'e, Database = Any>
{
    let mut query = sqlx::query(sql);
    for arg in arguments {
        query = bind_json_value(query, arg);
    }
    let res = query.execute(executor).await.map_err(|e| Error::Database(e.to_string()))?;
    Ok(AnyQueryResult {
        rows_affected: res.rows_affected(),
        last_insert_id: res.last_insert_id(),
    })
}

async fn fetch_all_sqlx<'e, E>(executor: E, sql: &str, arguments: &[Value]) -> Result<Vec<AnyRow>, Error>
where E: SqlxExecutor<'e, Database = Any>
{
    let mut query = sqlx::query(sql);
    for arg in arguments {
        query = bind_json_value(query, arg);
    }
    let rows = query.fetch_all(executor).await.map_err(|e| Error::Database(e.to_string()))?;
    Ok(rows.into_iter().map(sqlx_row_to_any_row).collect())
}

async fn fetch_optional_sqlx<'e, E>(executor: E, sql: &str, arguments: &[Value]) -> Result<Option<AnyRow>, Error>
where E: SqlxExecutor<'e, Database = Any>
{
    let mut query = sqlx::query(sql);
    for arg in arguments {
        query = bind_json_value(query, arg);
    }
    let row = query.fetch_optional(executor).await.map_err(|e| Error::Database(e.to_string()))?;
    Ok(row.map(sqlx_row_to_any_row))
}

async fn fetch_one_sqlx<'e, E>(executor: E, sql: &str, arguments: &[Value]) -> Result<AnyRow, Error>
where E: SqlxExecutor<'e, Database = Any>
{
    let mut query = sqlx::query(sql);
    for arg in arguments {
        query = bind_json_value(query, arg);
    }
    let row = query.fetch_one(executor).await.map_err(|e| Error::Database(e.to_string()))?;
    Ok(sqlx_row_to_any_row(row))
}

fn bind_json_value<'q>(query: sqlx::query::Query<'q, Any, sqlx::any::AnyArguments<'q>>, val: &'q Value) -> sqlx::query::Query<'q, Any, sqlx::any::AnyArguments<'q>> {
    match val {
        Value::Null => query.bind(None::<String>),
        Value::Bool(b) => query.bind(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                query.bind(i)
            } else if let Some(f) = n.as_f64() {
                query.bind(f)
            } else {
                query.bind(0.0)
            }
        }
        Value::String(s) => query.bind(s.as_str()),
        _ => query.bind(val.to_string()),
    }
}

fn sqlx_row_to_any_row(row: SqlxAnyRow) -> AnyRow {
    let mut columns = Vec::new();
    let mut values = Vec::new();

    for col in row.columns() {
        columns.push(AnyColumn {
            name: col.name().to_string(),
            type_info: AnyTypeInfo {
                name: col.type_info().name().to_string(),
            },
        });
        
        let val: DbValue = match row.try_get_raw(col.ordinal()) {
            Ok(raw_val) => {
                if raw_val.is_null() {
                    DbValue::Null
                } else {
                    if let Ok(v) = row.try_get::<i64, _>(col.ordinal()) {
                        DbValue::Integer(v)
                    } else if let Ok(v) = row.try_get::<f64, _>(col.ordinal()) {
                        DbValue::Real(v)
                    } else if let Ok(v) = row.try_get::<bool, _>(col.ordinal()) {
                        DbValue::Bool(v)
                    } else if let Ok(v) = row.try_get::<String, _>(col.ordinal()) {
                        DbValue::Text(v)
                    } else if let Ok(v) = row.try_get::<Vec<u8>, _>(col.ordinal()) {
                        DbValue::Blob(v)
                    } else {
                        DbValue::Text(format!("{:?}", raw_val))
                    }
                }
            }
            Err(_) => DbValue::Null,
        };
        values.push(val);
    }

    AnyRow { columns, values }
}
