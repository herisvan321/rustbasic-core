use crate::sql::AnyPool;

/// Mengganti placeholder Postgres ($1, $2, ...) dengan placeholder MySQL (?).
/// Implementasi manual tanpa regex — iterasi karakter satu kali (O(n)).
fn replace_postgres_placeholders(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            // Konsumsi digit setelah '$' (jika ada)
            let has_digit = chars.peek().map_or(false, |c| c.is_ascii_digit());
            if has_digit {
                while chars.peek().map_or(false, |c| c.is_ascii_digit()) {
                    chars.next();
                }
                result.push('?');
            } else {
                result.push('$');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[derive(Clone, Debug)]
pub struct RustBasicSessionStore {
    pub pool: AnyPool,
}

impl RustBasicSessionStore {
    pub fn new(pool: AnyPool) -> Self {
        Self { pool }
    }

    async fn get_placeholder_query(&self, sql: &str) -> String {
        let is_mysql = if let Ok(conn) = self.pool.acquire().await {
            conn.backend_name() == "MySQL"
        } else {
            false
        };

        if is_mysql {
            replace_postgres_placeholders(sql)
        } else {
            sql.to_string()
        }
    }

    pub async fn load(&self, id: &str) -> Option<String> {
        let raw_query = "SELECT payload FROM sessions WHERE id = $1 AND last_activity > $2";
        let query = self.get_placeholder_query(raw_query).await;
        let now = crate::chrono::Utc::now().timestamp();
        
        let row_opt = crate::sql::query(&query)
            .bind(id)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();

        if let Some(row) = row_opt {
            if let Ok(s) = row.try_get::<String, _>(0) {
                return Some(s);
            }
            if let Ok(bytes) = row.try_get::<Vec<u8>, _>(0) {
                if let Ok(s) = String::from_utf8(bytes) {
                    return Some(s);
                }
            }
        }
        None
    }

    pub async fn store(&self, id: &str, session_json: &str, ip: &str) {
        let raw_delete_query = "DELETE FROM sessions WHERE id = $1";
        let delete_query = self.get_placeholder_query(raw_delete_query).await;
        let _ = crate::sql::query(&delete_query).bind(id).execute(&self.pool).await;

        let raw_insert_query = "INSERT INTO sessions (id, payload, last_activity, ip_address) VALUES ($1, $2, $3, $4)";
        let insert_query = self.get_placeholder_query(raw_insert_query).await;
        let expires = crate::chrono::Utc::now().timestamp() + 14 * 24 * 60 * 60; // 14 hari

        let _ = crate::sql::query(&insert_query)
            .bind(id)
            .bind(session_json)
            .bind(expires)
            .bind(ip)
            .execute(&self.pool)
            .await;
    }
}
