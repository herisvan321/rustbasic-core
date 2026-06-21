use crate::Config;
#[cfg(feature = "mysql")]
use crate::colored::Colorize;
use crate::sql::{self, AnyPool};
use serde_json::Value;
use serde::de::DeserializeOwned;

pub async fn connect(cfg: &Config) -> AnyPool {
    let db_url = if let Ok(url) = std::env::var("DATABASE_URL") {
        url
    } else if cfg.db_connection == "mysql" {
        format!(
            "mysql://{}:{}@{}:{}/{}",
            cfg.db_username, cfg.db_password, cfg.db_host, cfg.db_port, cfg.db_database
        )
    } else {
        format!("sqlite:database/{}.sqlite?mode=rwc", cfg.db_database)
    };

    sql::any::install_default_drivers();

    let db_url_ref: &str = &db_url;
    match AnyPool::connect(db_url_ref).await {
        Ok(pool) => pool,
        Err(e) => {
            let err_msg = e.to_string();
            #[cfg(feature = "mysql")]
            if (err_msg.contains("1049") || err_msg.contains("Unknown database")) && cfg.db_connection == "mysql" {
                println!("{}", "⚠️  Database tidak ditemukan. Mencoba membuat database baru...".yellow());
                
                let root_url = format!(
                    "mysql://{}:{}@{}:{}",
                    cfg.db_username, cfg.db_password, cfg.db_host, cfg.db_port
                );
                
                if let Ok(pool) = sql::MySqlPool::connect(&root_url).await {
                    let create_query = format!("CREATE DATABASE IF NOT EXISTS `{}`", cfg.db_database);
                    if sql::query(&create_query).execute(&pool).await.is_ok() {
                        println!("✅ Database '{}' berhasil dibuat.", cfg.db_database.green());
                        return AnyPool::connect(&db_url).await.expect("Gagal terhubung setelah membuat database");
                    }
                }
            }
            let _ = err_msg; // suppress unused warning when mysql feature is disabled
            panic!("Gagal terhubung ke database: {:?}", e);
        }
    }
}

// -------------------------------------------------------------
// 📑 Query Builder (DB)
// -------------------------------------------------------------

#[derive(Clone)]
pub enum WhereClause {
    Simple { column: String, operator: String, value: Value },
    Raw { sql: String, binds: Vec<Value> },
}

#[derive(Clone)]
pub struct OrderClause {
    column: String,
    direction: String,
}

#[derive(Clone)]
pub struct QueryBuilder<'a> {
    pool: &'a AnyPool,
    table: String,
    wheres: Vec<WhereClause>,
    orders: Vec<OrderClause>,
    limit: Option<usize>,
}

pub struct DB;

impl DB {
    pub fn table<'a>(pool: &'a AnyPool, name: &str) -> QueryBuilder<'a> {
        QueryBuilder::new(pool, name)
    }
}

impl<'a> QueryBuilder<'a> {
    pub fn new(pool: &'a AnyPool, table: &str) -> Self {
        Self {
            pool,
            table: table.to_string(),
            wheres: Vec::new(),
            orders: Vec::new(),
            limit: None,
        }
    }

    pub fn where_(mut self, column: &str, value: impl serde::Serialize) -> Self {
        let val = serde_json::to_value(value).unwrap_or(Value::Null);
        self.wheres.push(WhereClause::Simple {
            column: column.to_string(),
            operator: "=".to_string(),
            value: val,
        });
        self
    }

    pub fn where_op(mut self, column: &str, operator: &str, value: impl serde::Serialize) -> Self {
        let val = serde_json::to_value(value).unwrap_or(Value::Null);
        self.wheres.push(WhereClause::Simple {
            column: column.to_string(),
            operator: operator.to_string(),
            value: val,
        });
        self
    }

    pub fn where_raw(mut self, sql: &str, binds: Vec<Value>) -> Self {
        self.wheres.push(WhereClause::Raw {
            sql: sql.to_string(),
            binds,
        });
        self
    }

    pub fn where_in(self, column: &str, values: Vec<impl serde::Serialize>) -> Self {
        if values.is_empty() {
            return self.where_raw("1 = 0", vec![]);
        }
        let placeholders: Vec<&str> = (0..values.len()).map(|_| "?").collect();
        let sql = format!("`{}` IN ({})", column, placeholders.join(", "));
        let binds: Vec<Value> = values.iter()
            .map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
            .collect();
        self.where_raw(&sql, binds)
    }

    pub fn pool(&self) -> &'a AnyPool {
        self.pool
    }


    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.orders.push(OrderClause {
            column: column.to_string(),
            direction: direction.to_string(),
        });
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    fn to_select_sql(&self) -> (String, Vec<Value>) {
        let mut sql = format!("SELECT * FROM `{}`", self.table);
        let mut binds = Vec::new();

        if !self.wheres.is_empty() {
            sql.push_str(" WHERE ");
            let mut parts = Vec::new();
            for w in &self.wheres {
                match w {
                    WhereClause::Simple { column, operator, value } => {
                        parts.push(format!("`{}` {} ?", column, operator));
                        binds.push(value.clone());
                    }
                    WhereClause::Raw { sql: raw_sql, binds: raw_binds } => {
                        parts.push(raw_sql.clone());
                        binds.extend(raw_binds.clone());
                    }
                }
            }
            sql.push_str(&parts.join(" AND "));
        }

        if !self.orders.is_empty() {
            sql.push_str(" ORDER BY ");
            let parts: Vec<String> = self.orders.iter()
                .map(|o| format!("`{}` {}", o.column, o.direction))
                .collect();
            sql.push_str(&parts.join(", "));
        }

        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        (sql, binds)
    }

    pub async fn first<T: DeserializeOwned>(&self) -> Result<Option<T>, sql::Error> {
        let mut builder = self.clone();
        builder.limit = Some(1);
        let (sql, binds) = builder.to_select_sql();

        let mut query = sql::query(&sql);
        for b in &binds {
            query = bind_query_json(query, b);
        }

        let row_opt = query.fetch_optional(self.pool).await?;
        if let Some(row) = row_opt {
            let val = row_to_json_value(&row);
            let parsed = serde_json::from_value::<T>(val)
                .map_err(|e| sql::Error::Protocol(format!("Deserialization error: {}", e)))?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    pub async fn get<T: DeserializeOwned>(&self) -> Result<Vec<T>, sql::Error> {
        let (sql, binds) = self.to_select_sql();

        let mut query = sql::query(&sql);
        for b in &binds {
            query = bind_query_json(query, b);
        }

        let rows = query.fetch_all(self.pool).await?;
        let mut result = Vec::new();
        for row in rows {
            let val = row_to_json_value(&row);
            let parsed = serde_json::from_value::<T>(val)
                .map_err(|e| sql::Error::Protocol(format!("Deserialization error: {}", e)))?;
            result.push(parsed);
        }
        Ok(result)
    }

    pub async fn count(&self) -> Result<i64, sql::Error> {
        let mut sql = format!("SELECT COUNT(*) FROM `{}`", self.table);
        let mut binds = Vec::new();

        if !self.wheres.is_empty() {
            sql.push_str(" WHERE ");
            let mut parts = Vec::new();
            for w in &self.wheres {
                match w {
                    WhereClause::Simple { column, operator, value } => {
                        parts.push(format!("`{}` {} ?", column, operator));
                        binds.push(value.clone());
                    }
                    WhereClause::Raw { sql: raw_sql, binds: raw_binds } => {
                        parts.push(raw_sql.clone());
                        binds.extend(raw_binds.clone());
                    }
                }
            }
            sql.push_str(&parts.join(" AND "));
        }

        let mut query = sql::query(&sql);
        for b in &binds {
            query = bind_query_json(query, b);
        }

        let row = query.fetch_one(self.pool).await?;
        let count_val: i64 = row.try_get(0).unwrap_or(0);
        Ok(count_val)
    }

    pub async fn insert(&self, data: Value) -> Result<(), sql::Error> {
        let obj = data.as_object().ok_or_else(|| {
            sql::Error::Protocol("Data insert harus berupa JSON object".into())
        })?;

        let mut columns = Vec::new();
        let mut placeholders = Vec::new();
        let mut binds = Vec::new();

        for (col, val) in obj {
            columns.push(format!("`{}`", col));
            placeholders.push("?");
            binds.push(val.clone());
        }

        let sql = format!(
            "INSERT INTO `{}` ({}) VALUES ({})",
            self.table,
            columns.join(", "),
            placeholders.join(", ")
        );

        let mut query = sql::query(&sql);
        for b in &binds {
            query = bind_query_json(query, b);
        }

        query.execute(self.pool).await?;
        Ok(())
    }

    pub async fn insert_get_id(&self, data: Value) -> Result<i64, sql::Error> {
        let obj = data.as_object().ok_or_else(|| {
            sql::Error::Protocol("Data insert harus berupa JSON object".into())
        })?;

        let mut columns = Vec::new();
        let mut placeholders = Vec::new();
        let mut binds = Vec::new();

        for (col, val) in obj {
            columns.push(format!("`{}`", col));
            placeholders.push("?");
            binds.push(val.clone());
        }

        let sql = format!(
            "INSERT INTO `{}` ({}) VALUES ({})",
            self.table,
            columns.join(", "),
            placeholders.join(", ")
        );

        let mut conn = self.pool.acquire().await?;

        let mut query = sql::query(&sql);
        for b in &binds {
            query = bind_query_json(query, b);
        }

        let result = query.execute(&mut *conn).await?;
        if let Some(id) = result.last_insert_id()
            && id != 0 {
                return Ok(id);
            }
        
        // Fallback for SQLite when using SQLx Any driver
        if let Ok(row) = sql::query("SELECT last_insert_rowid()").fetch_one(&mut *conn).await {
            let id: i64 = row.try_get(0).unwrap_or(0);
            if id != 0 {
                return Ok(id);
            }
        }
        
        // Fallback for MySQL when using SQLx Any driver
        if let Ok(row) = sql::query("SELECT LAST_INSERT_ID()").fetch_one(&mut *conn).await {
            let id: i64 = row.try_get(0).unwrap_or(0);
            if id != 0 {
                return Ok(id);
            }
        }

        Ok(0)
    }

    pub async fn update(&self, data: Value) -> Result<u64, sql::Error> {
        let obj = data.as_object().ok_or_else(|| {
            sql::Error::Protocol("Data update harus berupa JSON object".into())
        })?;

        let mut sets = Vec::new();
        let mut binds = Vec::new();

        for (col, val) in obj {
            sets.push(format!("`{}` = ?", col));
            binds.push(val.clone());
        }

        let mut sql = format!("UPDATE `{}` SET {}", self.table, sets.join(", "));

        if !self.wheres.is_empty() {
            sql.push_str(" WHERE ");
            let mut parts = Vec::new();
            for w in &self.wheres {
                match w {
                    WhereClause::Simple { column, operator, value } => {
                        parts.push(format!("`{}` {} ?", column, operator));
                        binds.push(value.clone());
                    }
                    WhereClause::Raw { sql: raw_sql, binds: raw_binds } => {
                        parts.push(raw_sql.clone());
                        binds.extend(raw_binds.clone());
                    }
                }
            }
            sql.push_str(&parts.join(" AND "));
        }

        let mut query = sql::query(&sql);
        for b in &binds {
            query = bind_query_json(query, b);
        }

        let result = query.execute(self.pool).await?;
        Ok(result.rows_affected())
    }

    pub async fn delete(&self) -> Result<u64, sql::Error> {
        let mut sql = format!("DELETE FROM `{}`", self.table);
        let mut binds = Vec::new();

        if !self.wheres.is_empty() {
            sql.push_str(" WHERE ");
            let mut parts = Vec::new();
            for w in &self.wheres {
                match w {
                    WhereClause::Simple { column, operator, value } => {
                        parts.push(format!("`{}` {} ?", column, operator));
                        binds.push(value.clone());
                    }
                    WhereClause::Raw { sql: raw_sql, binds: raw_binds } => {
                        parts.push(raw_sql.clone());
                        binds.extend(raw_binds.clone());
                    }
                }
            }
            sql.push_str(&parts.join(" AND "));
        }

        let mut query = sql::query(&sql);
        for b in &binds {
            query = bind_query_json(query, b);
        }

        let result = query.execute(self.pool).await?;
        Ok(result.rows_affected())
    }
}

// -------------------------------------------------------------
// Helper Value Binding & JSON mapping
// -------------------------------------------------------------

fn bind_query_json<'q>(
    query: sql::query::Query<'q, sql::Any, sql::any::AnyArguments<'q>>,
    val: &Value,
) -> sql::query::Query<'q, sql::Any, sql::any::AnyArguments<'q>> {
    match val {
        Value::Null => query.bind(None::<String>),
        Value::Bool(b) => query.bind(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                query.bind(i)
            } else if let Some(f) = n.as_f64() {
                query.bind(f)
            } else {
                query.bind(0.0f64)
            }
        }
        Value::String(s) => query.bind(s.clone()),
        _ => query.bind(val.to_string()),
    }
}

pub fn row_to_json_value(row: &sql::any::AnyRow) -> Value {
    let mut map = serde_json::Map::new();
    for i in 0..row.len() {
        let col = row.column(i);
        let name = col.name();
        let val = get_json_value(row, i);
        map.insert(name.to_string(), val);
    }
    Value::Object(map)
}

fn get_json_value(row: &sql::any::AnyRow, index: usize) -> Value {
    let type_name = row.column(index).type_info().name();
    if type_name == "NULL" {
        return Value::Null;
    }

    let type_name_upper = type_name.to_uppercase();
    if type_name_upper.contains("DATETIME") || type_name_upper.contains("TIMESTAMP") || type_name_upper.contains("DATE") || type_name_upper.contains("TIME") {
        if let Ok(Some(s)) = row.try_get::<Option<String>, _>(index) {
            return Value::String(s);
        }
        if let Ok(Some(bytes)) = row.try_get::<Option<Vec<u8>>, _>(index)
            && let Ok(s) = String::from_utf8(bytes) {
                return Value::String(s);
            }
        // If SQLx Any driver fails to decode, return Null instead of throwing/panicking
        return Value::Null;
    }

    // IMPORTANT: Integer harus dicek SEBELUM bool.
    // Di MySQL, SQLx Any driver bisa decode kolom INT sebagai bool (1 → true),
    // yang menyebabkan id=1 terbaca sebagai true dan gagal deserialisasi ke i32.
    if let Ok(Some(i)) = row.try_get::<Option<i64>, _>(index) {
        return Value::Number(serde_json::Number::from(i));
    }

    if let Ok(Some(f)) = row.try_get::<Option<f64>, _>(index)
        && let Some(num) = serde_json::Number::from_f64(f) {
            return Value::Number(num);
        }

    if let Ok(Some(b)) = row.try_get::<Option<bool>, _>(index) {
        return Value::Bool(b);
    }

    if let Ok(Some(s)) = row.try_get::<Option<String>, _>(index) {
        return Value::String(s);
    }

    if let Ok(Some(bytes)) = row.try_get::<Option<Vec<u8>>, _>(index)
        && let Ok(s) = String::from_utf8(bytes) {
            return Value::String(s);
        }

    Value::Null
}
