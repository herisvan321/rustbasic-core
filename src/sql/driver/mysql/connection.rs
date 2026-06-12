use std::net::TcpStream;
use crate::sql::driver::error::SqlError;
use crate::sql::driver::{SqlValue, SqlColumn, SqlRow, QueryResult};
use crate::sql::driver::mysql::protocol::{
    read_packet, write_packet, read_null_terminated_str,
    mysql_native_password_hash, caching_sha2_password_hash, parse_err_payload
};

#[derive(Debug)]
pub struct MySqlConnection {
    pub stream: TcpStream,
    pub capabilities: u32,
}

#[derive(Debug, Clone)]
struct MySqlColumn {
    pub name: String,
    pub column_type: u8,
}

impl MySqlConnection {
    pub fn connect(
        host: &str,
        port: u16,
        user: &str,
        password: &str,
        database: &str,
    ) -> Result<Self, SqlError> {
        let mut stream = TcpStream::connect(format!("{}:{}", host, port))?;
        
        // Read Initial Handshake
        let (payload, _seq) = read_packet(&mut stream)?;
        if payload.is_empty() {
            return Err(SqlError::Protocol("Empty handshake packet".into()));
        }
        if payload[0] == 0xFF {
            return Err(parse_err_payload(&payload));
        }
        
        let mut cursor = &payload[..];
        let _protocol_ver = cursor[0];
        cursor = &cursor[1..];
        
        let _server_ver = read_null_terminated_str(&mut cursor)?;
        let _connection_id = u32::from_le_bytes([cursor[0], cursor[1], cursor[2], cursor[3]]);
        cursor = &cursor[4..];
        
        let mut salt = Vec::new();
        salt.extend_from_slice(&cursor[0..8]);
        cursor = &cursor[8..];
        
        let _filler = cursor[0];
        cursor = &cursor[1..];
        
        let cap_lower = u16::from_le_bytes([cursor[0], cursor[1]]) as u32;
        cursor = &cursor[2..];
        
        let _charset = cursor[0];
        cursor = &cursor[1..];
        
        let _status = u16::from_le_bytes([cursor[0], cursor[1]]);
        cursor = &cursor[2..];
        
        let cap_upper = u16::from_le_bytes([cursor[0], cursor[1]]) as u32;
        cursor = &cursor[2..];
        
        let server_capabilities = cap_lower | (cap_upper << 16);
        
        let mut auth_len = 0;
        if (server_capabilities & 0x00080000) != 0 { // CLIENT_PLUGIN_AUTH
            auth_len = cursor[0];
            cursor = &cursor[1..];
        } else {
            cursor = &cursor[1..];
        }
        
        // Reserved 10 bytes
        cursor = &cursor[10..];
        
        if (server_capabilities & 0x00008000) != 0 { // CLIENT_SECURE_CONNECTION
            let salt2_len = std::cmp::max(13, auth_len as usize - 8);
            if cursor.len() >= salt2_len {
                let salt2 = &cursor[..salt2_len];
                let salt2_clean = if salt2.last() == Some(&0) {
                    &salt2[..salt2.len() - 1]
                } else {
                    salt2
                };
                salt.extend_from_slice(salt2_clean);
                cursor = &cursor[salt2_len..];
            }
        }
        
        let mut auth_plugin = String::new();
        if (server_capabilities & 0x00080000) != 0 && !cursor.is_empty() { // CLIENT_PLUGIN_AUTH
            auth_plugin = read_null_terminated_str(&mut cursor).unwrap_or_default();
        }
        
        // Truncate salt to 20 bytes if mysql_native_password is used
        if auth_plugin == "mysql_native_password" || auth_plugin.is_empty() {
            salt.truncate(20);
        }
        
        // Build Handshake Response
        let client_capabilities: u32 = 0x200 | 0x8000 | 0x80000 | 0x08; // CLIENT_PROTOCOL_41 | CLIENT_SECURE_CONNECTION | CLIENT_PLUGIN_AUTH | CLIENT_CONNECT_WITH_DB
        
        let mut resp = Vec::new();
        resp.extend_from_slice(&client_capabilities.to_le_bytes());
        resp.extend_from_slice(&[0xff, 0xff, 0xff, 0x00]); // Max packet size (16MB)
        resp.push(45); // Charset utf8mb4_general_ci
        resp.extend_from_slice(&[0u8; 23]); // Reserved
        
        // Username: null terminated
        resp.extend_from_slice(user.as_bytes());
        resp.push(0);
        
        // Auth response
        if password.is_empty() {
            resp.push(0);
        } else {
            if auth_plugin == "mysql_native_password" || auth_plugin.is_empty() {
                let auth_data = mysql_native_password_hash(password.as_bytes(), &salt);
                resp.push(auth_data.len() as u8);
                resp.extend_from_slice(&auth_data);
            } else if auth_plugin == "caching_sha2_password" {
                let auth_data = caching_sha2_password_hash(password.as_bytes(), &salt);
                resp.push(auth_data.len() as u8);
                resp.extend_from_slice(&auth_data);
            } else {
                resp.push(password.len() as u8);
                resp.extend_from_slice(password.as_bytes());
            }
        }
        
        // Database: null terminated
        resp.extend_from_slice(database.as_bytes());
        resp.push(0);
        
        // Auth plugin name: null terminated
        if !auth_plugin.is_empty() {
            resp.extend_from_slice(auth_plugin.as_bytes());
            resp.push(0);
        } else {
            resp.extend_from_slice(b"mysql_native_password");
            resp.push(0);
        }
        
        // Handshake Response is seq_id = 1
        write_packet(&mut stream, 1, &resp)?;
        
        // Read response
        let (ok_payload, seq) = read_packet(&mut stream)?;
        if ok_payload.is_empty() {
            return Err(SqlError::Protocol("Empty response after handshake response".into()));
        }
        
        if ok_payload[0] == 0xFF {
            return Err(parse_err_payload(&ok_payload));
        }
        
        // Handle Auth Switch Request
        if ok_payload[0] == 0xFE {
            let mut cur = &ok_payload[1..];
            let new_plugin = read_null_terminated_str(&mut cur)?;
            let mut new_salt = cur.to_vec();
            
            if new_plugin == "mysql_native_password" {
                new_salt.truncate(20);
                let hash = mysql_native_password_hash(password.as_bytes(), &new_salt);
                write_packet(&mut stream, seq + 1, &hash)?;
                
                let (final_payload, _) = read_packet(&mut stream)?;
                if final_payload[0] == 0xFF {
                    return Err(parse_err_payload(&final_payload));
                }
            } else if new_plugin == "caching_sha2_password" {
                let hash = caching_sha2_password_hash(password.as_bytes(), &new_salt);
                write_packet(&mut stream, seq + 1, &hash)?;
                
                let (next_payload, next_seq) = read_packet(&mut stream)?;
                if next_payload.is_empty() {
                    return Err(SqlError::Protocol("Empty response after auth switch response".into()));
                }
                if next_payload[0] == 0xFF {
                    return Err(parse_err_payload(&next_payload));
                }
                
                if next_payload[0] == 0x01 {
                    if next_payload.len() > 1 && next_payload[1] == 3 {
                        // Fast path succeeded. Next packet is OK or ERR.
                        let (ok2, _) = read_packet(&mut stream)?;
                        if ok2[0] == 0xFF {
                            return Err(parse_err_payload(&ok2));
                        }
                    } else if next_payload.len() > 1 && next_payload[1] == 4 {
                        // Full auth required: Request public key
                        write_packet(&mut stream, next_seq + 1, &[0x02])?;
                        let (pk_payload, final_seq) = read_packet(&mut stream)?;
                        if pk_payload[0] == 0xFF {
                            return Err(parse_err_payload(&pk_payload));
                        }
                        let pub_key_str = String::from_utf8_lossy(&pk_payload).into_owned();
                        let start_idx = pub_key_str.find("-----BEGIN PUBLIC KEY-----").unwrap_or(0);
                        let pub_key_pem = &pub_key_str[start_idx..];
                        let encrypted = crate::sql::driver::mysql::protocol::rsa_encrypt_password(password, &new_salt, pub_key_pem)?;
                        write_packet(&mut stream, final_seq + 1, &encrypted)?;
                        let (final_payload, _) = read_packet(&mut stream)?;
                        if final_payload[0] == 0xFF {
                            return Err(parse_err_payload(&final_payload));
                        }
                    }
                }
            } else if new_plugin == "mysql_clear_password" {
                let mut clear_pwd = password.as_bytes().to_vec();
                clear_pwd.push(0);
                write_packet(&mut stream, seq + 1, &clear_pwd)?;
                
                let (final_payload, _) = read_packet(&mut stream)?;
                if final_payload[0] == 0xFF {
                    return Err(parse_err_payload(&final_payload));
                }
            } else {
                return Err(SqlError::Protocol(format!("Unsupported auth switch to: {}", new_plugin)));
            }
        } else if ok_payload[0] == 0x01 && (auth_plugin == "caching_sha2_password" || auth_plugin.is_empty()) {
            // caching_sha2_password fast auth path response
            if ok_payload.len() > 1 && ok_payload[1] == 3 {
                // Fast path succeeded. Next packet is OK or ERR.
                let (ok2, _) = read_packet(&mut stream)?;
                if ok2[0] == 0xFF {
                    return Err(parse_err_payload(&ok2));
                }
            } else if ok_payload.len() > 1 && ok_payload[1] == 4 {
                // Full auth required. Request public key.
                write_packet(&mut stream, seq + 1, &[0x02])?;
                let (pk_payload, next_seq) = read_packet(&mut stream)?;
                if pk_payload.is_empty() {
                    return Err(SqlError::Protocol("Empty public key payload".into()));
                }
                if pk_payload[0] == 0xFF {
                    return Err(parse_err_payload(&pk_payload));
                }
                
                let pub_key_str = String::from_utf8_lossy(&pk_payload).into_owned();
                let start_idx = pub_key_str.find("-----BEGIN PUBLIC KEY-----").unwrap_or(0);
                let pub_key_pem = &pub_key_str[start_idx..];
                let encrypted = crate::sql::driver::mysql::protocol::rsa_encrypt_password(password, &salt, pub_key_pem)?;
                
                // Send encrypted password
                write_packet(&mut stream, next_seq + 1, &encrypted)?;
                
                // Read final response (OK/ERR)
                let (final_payload, _) = read_packet(&mut stream)?;
                if final_payload.is_empty() {
                    return Err(SqlError::Protocol("Empty final auth response".into()));
                }
                if final_payload[0] == 0xFF {
                    return Err(parse_err_payload(&final_payload));
                }
            }
        }
        
        Ok(MySqlConnection {
            stream,
            capabilities: server_capabilities,
        })
    }
    
    pub fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Result<QueryResult, SqlError> {
        let interpolated = interpolate_query(sql, params)?;
        
        let mut payload = Vec::new();
        payload.push(0x03); // COM_QUERY
        payload.extend_from_slice(interpolated.as_bytes());
        
        // COM_QUERY always starts with sequence ID = 0
        write_packet(&mut self.stream, 0, &payload)?;
        
        let (resp_payload, _) = read_packet(&mut self.stream)?;
        if resp_payload.is_empty() {
            return Err(SqlError::Protocol("Empty response from query execution".into()));
        }
        
        if resp_payload[0] == 0xFF {
            return Err(parse_err_payload(&resp_payload));
        }
        
        if resp_payload[0] == 0x00 || resp_payload[0] == 0xFE {
            let mut cursor = &resp_payload[1..];
            let affected_rows = crate::sql::driver::mysql::protocol::read_lenenc_int(&mut cursor)?.unwrap_or(0);
            let last_insert_id = crate::sql::driver::mysql::protocol::read_lenenc_int(&mut cursor)?.unwrap_or(0);
            return Ok(QueryResult {
                rows_affected: affected_rows,
                last_insert_id,
            });
        }
        
        // If it starts with anything else, it's a resultset column count (unexpected).
        // Drain packets to keep connection in sync.
        let mut cursor = &resp_payload[..];
        let col_count = crate::sql::driver::mysql::protocol::read_lenenc_int(&mut cursor)?.unwrap_or(0) as usize;
        
        for _ in 0..col_count {
            let _ = read_packet(&mut self.stream)?;
        }
        let _ = read_packet(&mut self.stream)?; // Definitions EOF
        
        loop {
            let (row_payload, _) = read_packet(&mut self.stream)?;
            if row_payload.is_empty() {
                break;
            }
            if row_payload[0] == 0xFF {
                return Err(parse_err_payload(&row_payload));
            }
            if row_payload[0] == 0xFE && row_payload.len() < 9 {
                break; // Rows EOF
            }
        }
        
        Ok(QueryResult {
            rows_affected: 0,
            last_insert_id: 0,
        })
    }
    
    pub fn query(&mut self, sql: &str, params: &[SqlValue]) -> Result<Vec<SqlRow>, SqlError> {
        let interpolated = interpolate_query(sql, params)?;
        
        let mut payload = Vec::new();
        payload.push(0x03); // COM_QUERY
        payload.extend_from_slice(interpolated.as_bytes());
        
        write_packet(&mut self.stream, 0, &payload)?;
        
        let (resp_payload, _) = read_packet(&mut self.stream)?;
        if resp_payload.is_empty() {
            return Err(SqlError::Protocol("Empty response from query execution".into()));
        }
        
        if resp_payload[0] == 0xFF {
            return Err(parse_err_payload(&resp_payload));
        }
        
        if resp_payload[0] == 0x00 || resp_payload[0] == 0xFE {
            return Ok(Vec::new());
        }
        
        let mut cursor = &resp_payload[..];
        let col_count = crate::sql::driver::mysql::protocol::read_lenenc_int(&mut cursor)?.ok_or_else(|| {
            SqlError::Protocol("Invalid column count in resultset".into())
        })? as usize;
        
        let mut columns = Vec::with_capacity(col_count);
        for _ in 0..col_count {
            let (col_payload, _) = read_packet(&mut self.stream)?;
            if col_payload.is_empty() {
                return Err(SqlError::Protocol("Empty column definition packet".into()));
            }
            if col_payload[0] == 0xFF {
                return Err(parse_err_payload(&col_payload));
            }
            let col = parse_column_definition(&col_payload)?;
            columns.push(col);
        }
        
        // Read EOF packet
        let (eof_payload, _) = read_packet(&mut self.stream)?;
        if eof_payload.is_empty() {
            return Err(SqlError::Protocol("Empty column definitions EOF packet".into()));
        }
        if eof_payload[0] == 0xFF {
            return Err(parse_err_payload(&eof_payload));
        }
        
        let mut rows = Vec::new();
        loop {
            let (row_payload, _) = read_packet(&mut self.stream)?;
            if row_payload.is_empty() {
                return Err(SqlError::Protocol("Empty row packet".into()));
            }
            if row_payload[0] == 0xFF {
                return Err(parse_err_payload(&row_payload));
            }
            if row_payload[0] == 0xFE && row_payload.len() < 9 {
                break; // EOF
            }
            
            let mut row_cursor = &row_payload[..];
            let mut values = Vec::with_capacity(col_count);
            for col in &columns {
                if row_cursor.is_empty() {
                    return Err(SqlError::Protocol("Truncated row packet".into()));
                }
                let next_byte = row_cursor[0];
                if next_byte == 0xFB {
                    values.push(SqlValue::Null);
                    row_cursor = &row_cursor[1..];
                } else {
                    let val_bytes = crate::sql::driver::mysql::protocol::read_lenenc_bytes(&mut row_cursor)?.ok_or_else(|| {
                        SqlError::Protocol("Expected length-encoded bytes in row".into())
                    })?;
                    
                    let is_integer = match col.column_type {
                        1 | 2 | 3 | 8 | 9 => true,
                        _ => false,
                    };
                    let is_real = match col.column_type {
                        4 | 5 => true,
                        _ => false,
                    };
                    
                    let value = if let Ok(s) = String::from_utf8(val_bytes.clone()) {
                        if is_integer {
                            if let Ok(i) = s.parse::<i64>() {
                                SqlValue::Integer(i)
                            } else {
                                SqlValue::Text(s)
                            }
                        } else if is_real {
                            if let Ok(f) = s.parse::<f64>() {
                                SqlValue::Real(f)
                            } else {
                                SqlValue::Text(s)
                            }
                        } else {
                            SqlValue::Text(s)
                        }
                    } else {
                        SqlValue::Blob(val_bytes)
                    };
                    values.push(value);
                }
            }
            let sql_columns = columns.iter().map(|c| SqlColumn { name: c.name.clone() }).collect();
            rows.push(SqlRow { columns: sql_columns, values });
        }
        
        Ok(rows)
    }

    pub fn begin(&mut self) -> Result<MySqlTransaction<'_>, SqlError> {
        self.execute("BEGIN", &[])?;
        Ok(MySqlTransaction {
            conn: self,
            committed: false,
        })
    }
}

fn parse_column_definition(payload: &[u8]) -> Result<MySqlColumn, SqlError> {
    let mut cursor = &payload[..];
    let _catalog = crate::sql::driver::mysql::protocol::read_lenenc_str(&mut cursor)?;
    let _schema = crate::sql::driver::mysql::protocol::read_lenenc_str(&mut cursor)?;
    let _table = crate::sql::driver::mysql::protocol::read_lenenc_str(&mut cursor)?;
    let _org_table = crate::sql::driver::mysql::protocol::read_lenenc_str(&mut cursor)?;
    let name = crate::sql::driver::mysql::protocol::read_lenenc_str(&mut cursor)?.ok_or_else(|| {
        SqlError::Protocol("Column name is null".into())
    })?;
    let _org_name = crate::sql::driver::mysql::protocol::read_lenenc_str(&mut cursor)?;
    
    let _fixed_len = crate::sql::driver::mysql::protocol::read_lenenc_int(&mut cursor)?;
    if cursor.len() < 7 {
        return Err(SqlError::Protocol("Truncated column definition".into()));
    }
    let column_type = cursor[6]; // type byte is the 7th byte of fixed fields
    Ok(MySqlColumn { name, column_type })
}

pub fn interpolate_query(sql: &str, params: &[SqlValue]) -> Result<String, SqlError> {
    let mut result = String::new();
    let mut param_idx = 0;
    let mut in_string = false;
    let mut escape = false;
    
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if escape {
            result.push(c);
            escape = false;
            i += 1;
            continue;
        }
        if c == '\\' {
            result.push(c);
            escape = true;
            i += 1;
            continue;
        }
        if c == '\'' {
            in_string = !in_string;
            result.push(c);
            i += 1;
            continue;
        }
        if c == '?' && !in_string {
            if param_idx >= params.len() {
                return Err(SqlError::Protocol("Too few parameters provided for query".into()));
            }
            let param_str = format_param(&params[param_idx]);
            result.push_str(&param_str);
            param_idx += 1;
        } else {
            result.push(c);
        }
        i += 1;
    }
    
    if param_idx < params.len() {
        return Err(SqlError::Protocol("Too many parameters provided for query".into()));
    }
    
    Ok(result)
}

fn format_param(param: &SqlValue) -> String {
    match param {
        SqlValue::Null => "NULL".to_string(),
        SqlValue::Integer(i) => i.to_string(),
        SqlValue::Real(f) => f.to_string(),
        SqlValue::Text(s) => {
            let escaped = escape_string(s);
            format!("'{}'", escaped)
        }
        SqlValue::Blob(bytes) => {
            let hex: String = bytes.iter().map(|b| format!("{:02X}", b)).collect();
            format!("X'{}'", hex)
        }
    }
}

fn escape_string(s: &str) -> String {
    let mut escaped = String::new();
    for c in s.chars() {
        match c {
            '\'' => escaped.push_str("\\'"),
            '\"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\0' => escaped.push_str("\\0"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\x1a' => escaped.push_str("\\Z"),
            _ => escaped.push(c),
        }
    }
    escaped
}

pub struct MySqlTransaction<'a> {
    conn: &'a mut MySqlConnection,
    committed: bool,
}

impl<'a> MySqlTransaction<'a> {
    pub fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Result<QueryResult, SqlError> {
        self.conn.execute(sql, params)
    }

    pub fn query(&mut self, sql: &str, params: &[SqlValue]) -> Result<Vec<SqlRow>, SqlError> {
        self.conn.query(sql, params)
    }

    pub fn commit(mut self) -> Result<(), SqlError> {
        self.conn.execute("COMMIT", &[])?;
        self.committed = true;
        Ok(())
    }

    pub fn rollback(mut self) -> Result<(), SqlError> {
        self.conn.execute("ROLLBACK", &[])?;
        self.committed = true;
        Ok(())
    }
}

impl<'a> Drop for MySqlTransaction<'a> {
    fn drop(&mut self) {
        if !self.committed {
            let _ = self.conn.execute("ROLLBACK", &[]);
        }
    }
}

impl crate::sql::driver::SqlConnection for MySqlConnection {
    fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Result<QueryResult, SqlError> {
        self.execute(sql, params)
    }

    fn query(&mut self, sql: &str, params: &[SqlValue]) -> Result<Vec<SqlRow>, SqlError> {
        self.query(sql, params)
    }
}

impl<'a> crate::sql::driver::SqlConnection for MySqlTransaction<'a> {
    fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Result<QueryResult, SqlError> {
        self.execute(sql, params)
    }

    fn query(&mut self, sql: &str, params: &[SqlValue]) -> Result<Vec<SqlRow>, SqlError> {
        self.query(sql, params)
    }
}
