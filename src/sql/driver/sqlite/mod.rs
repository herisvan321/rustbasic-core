use std::ffi::{c_char, c_int, c_void, CStr, CString};
use crate::sql::driver::error::SqlError;
use crate::sql::driver::{SqlValue, SqlColumn, SqlRow, QueryResult};

// Opaque SQLite database and statement handles
#[repr(C)]
pub struct sqlite3 {
    _private: [u8; 0],
}

#[repr(C)]
pub struct sqlite3_stmt {
    _private: [u8; 0],
}

// SQLite Constants
const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_DONE: c_int = 101;

const SQLITE_INTEGER: c_int = 1;
const SQLITE_FLOAT: c_int = 2;
const SQLITE_TEXT: c_int = 3;
const SQLITE_BLOB: c_int = 4;
const SQLITE_NULL: c_int = 5;

#[link(name = "sqlite3")]
unsafe extern "C" {
    fn sqlite3_open(filename: *const c_char, ppDb: *mut *mut sqlite3) -> c_int;
    fn sqlite3_close(db: *mut sqlite3) -> c_int;
    fn sqlite3_errmsg(db: *mut sqlite3) -> *const c_char;
    
    fn sqlite3_prepare_v2(
        db: *mut sqlite3,
        zSql: *const c_char,
        nByte: c_int,
        ppStmt: *mut *mut sqlite3_stmt,
        pzTail: *mut *const c_char,
    ) -> c_int;
    
    fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_step(pStmt: *mut sqlite3_stmt) -> c_int;
    
    // Binding parameters
    fn sqlite3_bind_null(pStmt: *mut sqlite3_stmt, index: c_int) -> c_int;
    fn sqlite3_bind_int64(pStmt: *mut sqlite3_stmt, index: c_int, value: i64) -> c_int;
    fn sqlite3_bind_double(pStmt: *mut sqlite3_stmt, index: c_int, value: f64) -> c_int;
    fn sqlite3_bind_text(
        pStmt: *mut sqlite3_stmt,
        index: c_int,
        value: *const c_char,
        n: c_int,
        destructor: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> c_int;
    fn sqlite3_bind_blob(
        pStmt: *mut sqlite3_stmt,
        index: c_int,
        value: *const c_void,
        n: c_int,
        destructor: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> c_int;

    // Retrieving columns
    fn sqlite3_column_count(pStmt: *mut sqlite3_stmt) -> c_int;
    fn sqlite3_column_name(pStmt: *mut sqlite3_stmt, N: c_int) -> *const c_char;
    fn sqlite3_column_type(pStmt: *mut sqlite3_stmt, iCol: c_int) -> c_int;
    
    fn sqlite3_column_int64(pStmt: *mut sqlite3_stmt, iCol: c_int) -> i64;
    fn sqlite3_column_double(pStmt: *mut sqlite3_stmt, iCol: c_int) -> f64;
    fn sqlite3_column_text(pStmt: *mut sqlite3_stmt, iCol: c_int) -> *const c_char;
    fn sqlite3_column_blob(pStmt: *mut sqlite3_stmt, iCol: c_int) -> *const c_void;
    fn sqlite3_column_bytes(pStmt: *mut sqlite3_stmt, iCol: c_int) -> c_int;
    
    fn sqlite3_changes(db: *mut sqlite3) -> c_int;
    fn sqlite3_last_insert_rowid(db: *mut sqlite3) -> i64;
}

pub struct SqliteConnection {
    db: *mut sqlite3,
}

unsafe impl Send for SqliteConnection {}

impl Drop for SqliteConnection {
    fn drop(&mut self) {
        if !self.db.is_null() {
            unsafe {
                sqlite3_close(self.db);
            }
        }
    }
}

impl SqliteConnection {
    pub fn connect(path: &str) -> Result<Self, SqlError> {
        let c_path = CString::new(path)
            .map_err(|e| SqlError::Other(format!("Invalid path: {}", e)))?;
        let mut db = std::ptr::null_mut();
        let rc = unsafe { sqlite3_open(c_path.as_ptr(), &mut db) };
        if rc != SQLITE_OK {
            let err_msg = if db.is_null() {
                "Failed to open database".to_string()
            } else {
                unsafe {
                    let err = sqlite3_errmsg(db);
                    CStr::from_ptr(err).to_string_lossy().into_owned()
                }
            };
            if !db.is_null() {
                unsafe { sqlite3_close(db); }
            }
            return Err(SqlError::Other(err_msg));
        }
        Ok(Self { db })
    }

    fn prepare_and_bind(&self, sql: &str, params: &[SqlValue]) -> Result<*mut sqlite3_stmt, SqlError> {
        let c_sql = CString::new(sql)
            .map_err(|e| SqlError::Other(format!("Invalid SQL string: {}", e)))?;
        let mut stmt = std::ptr::null_mut();
        let rc = unsafe {
            sqlite3_prepare_v2(self.db, c_sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut())
        };
        if rc != SQLITE_OK {
            let err_msg = unsafe {
                let err = sqlite3_errmsg(self.db);
                CStr::from_ptr(err).to_string_lossy().into_owned()
            };
            return Err(SqlError::Other(err_msg));
        }

        let sqlite_transient: Option<unsafe extern "C" fn(*mut c_void)> = unsafe {
            std::mem::transmute(-1isize)
        };

        for (i, param) in params.iter().enumerate() {
            let idx = (i + 1) as c_int;
            let rc = unsafe {
                match param {
                    SqlValue::Null => sqlite3_bind_null(stmt, idx),
                    SqlValue::Integer(val) => sqlite3_bind_int64(stmt, idx, *val),
                    SqlValue::Real(val) => sqlite3_bind_double(stmt, idx, *val),
                    SqlValue::Text(val) => {
                        let c_str = CString::new(val.as_str()).map_err(|e| SqlError::Other(e.to_string()))?;
                        sqlite3_bind_text(stmt, idx, c_str.as_ptr(), -1, sqlite_transient)
                    }
                    SqlValue::Blob(val) => {
                        sqlite3_bind_blob(stmt, idx, val.as_ptr() as *const c_void, val.len() as c_int, sqlite_transient)
                    }
                }
            };
            if rc != SQLITE_OK {
                let err_msg = unsafe {
                    let err = sqlite3_errmsg(self.db);
                    CStr::from_ptr(err).to_string_lossy().into_owned()
                };
                unsafe { sqlite3_finalize(stmt); }
                return Err(SqlError::Other(err_msg));
            }
        }

        Ok(stmt)
    }

    pub fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Result<QueryResult, SqlError> {
        let stmt = self.prepare_and_bind(sql, params)?;
        let mut rc = unsafe { sqlite3_step(stmt) };
        
        while rc == SQLITE_ROW {
            rc = unsafe { sqlite3_step(stmt) };
        }

        if rc != SQLITE_DONE {
            let err_msg = unsafe {
                let err = sqlite3_errmsg(self.db);
                CStr::from_ptr(err).to_string_lossy().into_owned()
            };
            unsafe { sqlite3_finalize(stmt); }
            return Err(SqlError::Other(err_msg));
        }

        unsafe { sqlite3_finalize(stmt); }

        let rows_affected = unsafe { sqlite3_changes(self.db) } as u64;
        let last_insert_id = unsafe { sqlite3_last_insert_rowid(self.db) } as u64;

        Ok(QueryResult {
            rows_affected,
            last_insert_id,
        })
    }

    pub fn query(&mut self, sql: &str, params: &[SqlValue]) -> Result<Vec<SqlRow>, SqlError> {
        let stmt = self.prepare_and_bind(sql, params)?;
        
        let col_count = unsafe { sqlite3_column_count(stmt) } as usize;
        let mut columns = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let name_ptr = unsafe { sqlite3_column_name(stmt, i as c_int) };
            let name = if name_ptr.is_null() {
                format!("column_{}", i)
            } else {
                unsafe { CStr::from_ptr(name_ptr).to_string_lossy().into_owned() }
            };
            columns.push(SqlColumn { name });
        }

        let mut rows = Vec::new();
        loop {
            let rc = unsafe { sqlite3_step(stmt) };
            if rc == SQLITE_DONE {
                break;
            }
            if rc != SQLITE_ROW {
                let err_msg = unsafe {
                    let err = sqlite3_errmsg(self.db);
                    CStr::from_ptr(err).to_string_lossy().into_owned()
                };
                unsafe { sqlite3_finalize(stmt); }
                return Err(SqlError::Other(err_msg));
            }

            let mut values = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let col_idx = i as c_int;
                let val_type = unsafe { sqlite3_column_type(stmt, col_idx) };
                let val = match val_type {
                    SQLITE_NULL => SqlValue::Null,
                    SQLITE_INTEGER => {
                        let v = unsafe { sqlite3_column_int64(stmt, col_idx) };
                        SqlValue::Integer(v)
                    }
                    SQLITE_FLOAT => {
                        let v = unsafe { sqlite3_column_double(stmt, col_idx) };
                        SqlValue::Real(v)
                    }
                    SQLITE_TEXT => {
                        let text_ptr = unsafe { sqlite3_column_text(stmt, col_idx) };
                        let s = if text_ptr.is_null() {
                            String::new()
                        } else {
                            unsafe { CStr::from_ptr(text_ptr as *const c_char).to_string_lossy().into_owned() }
                        };
                        SqlValue::Text(s)
                    }
                    SQLITE_BLOB => {
                        let blob_ptr = unsafe { sqlite3_column_blob(stmt, col_idx) };
                        let num_bytes = unsafe { sqlite3_column_bytes(stmt, col_idx) } as usize;
                        let mut bytes = vec![0u8; num_bytes];
                        if !blob_ptr.is_null() && num_bytes > 0 {
                            unsafe {
                                std::ptr::copy_nonoverlapping(blob_ptr as *const u8, bytes.as_mut_ptr(), num_bytes);
                            }
                        }
                        SqlValue::Blob(bytes)
                    }
                    _ => SqlValue::Null,
                };
                values.push(val);
            }
            rows.push(SqlRow {
                columns: columns.clone(),
                values,
            });
        }

        unsafe { sqlite3_finalize(stmt); }
        Ok(rows)
    }

    pub fn begin(&mut self) -> Result<SqliteTransaction<'_>, SqlError> {
        self.execute("BEGIN", &[])?;
        Ok(SqliteTransaction {
            conn: self,
            committed: false,
        })
    }
}

pub struct SqliteTransaction<'a> {
    conn: &'a mut SqliteConnection,
    committed: bool,
}

impl<'a> SqliteTransaction<'a> {
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

impl<'a> Drop for SqliteTransaction<'a> {
    fn drop(&mut self) {
        if !self.committed {
            let _ = self.conn.execute("ROLLBACK", &[]);
        }
    }
}

impl crate::sql::driver::SqlConnection for SqliteConnection {
    fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Result<QueryResult, SqlError> {
        self.execute(sql, params)
    }

    fn query(&mut self, sql: &str, params: &[SqlValue]) -> Result<Vec<SqlRow>, SqlError> {
        self.query(sql, params)
    }
}

impl<'a> crate::sql::driver::SqlConnection for SqliteTransaction<'a> {
    fn execute(&mut self, sql: &str, params: &[SqlValue]) -> Result<QueryResult, SqlError> {
        self.execute(sql, params)
    }

    fn query(&mut self, sql: &str, params: &[SqlValue]) -> Result<Vec<SqlRow>, SqlError> {
        self.query(sql, params)
    }
}
