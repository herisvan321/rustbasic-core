use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use crate::sql::driver::mysql::connection::MySqlConnection;
use crate::sql::driver::error::SqlError;

#[derive(Clone)]
pub struct MySqlPool {
    inner: Arc<Mutex<PoolInner>>,
}

struct PoolInner {
    connections: VecDeque<MySqlConnection>,
    host: String,
    port: u16,
    user: String,
    pass: String,
    db: String,
}

pub struct PoolConnection {
    conn: Option<MySqlConnection>,
    pool: Arc<Mutex<PoolInner>>,
}

impl MySqlPool {
    pub fn new(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        database: &str,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PoolInner {
                connections: VecDeque::new(),
                host: host.to_string(),
                port,
                user: user.to_string(),
                pass: password.to_string(),
                db: database.to_string(),
            })),
        }
    }

    pub fn acquire(&self) -> Result<PoolConnection, SqlError> {
        let mut inner = self.inner.lock().unwrap();
        
        while let Some(mut conn) = inner.connections.pop_front() {
            // Check if connection is still healthy
            if conn.execute("SELECT 1", &[]).is_ok() {
                return Ok(PoolConnection {
                    conn: Some(conn),
                    pool: self.inner.clone(),
                });
            }
        }

        // None available or all dead, connect new
        let conn = MySqlConnection::connect(
            &inner.host,
            inner.port,
            &inner.user,
            &inner.pass,
            &inner.db,
        )?;

        Ok(PoolConnection {
            conn: Some(conn),
            pool: self.inner.clone(),
        })
    }
}

impl std::ops::Deref for PoolConnection {
    type Target = MySqlConnection;
    fn deref(&self) -> &Self::Target {
        self.conn.as_ref().unwrap()
    }
}

impl std::ops::DerefMut for PoolConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.conn.as_mut().unwrap()
    }
}

impl Drop for PoolConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            if let Ok(mut inner) = self.pool.lock() {
                inner.connections.push_back(conn);
            }
        }
    }
}

impl crate::sql::driver::SqlConnection for PoolConnection {
    fn execute(&mut self, sql: &str, params: &[crate::sql::driver::SqlValue]) -> Result<crate::sql::driver::QueryResult, SqlError> {
        self.conn.as_mut().unwrap().execute(sql, params)
    }

    fn query(&mut self, sql: &str, params: &[crate::sql::driver::SqlValue]) -> Result<Vec<crate::sql::driver::SqlRow>, SqlError> {
        self.conn.as_mut().unwrap().query(sql, params)
    }
}
