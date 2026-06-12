#[cfg(feature = "sqlite")]
#[test]
fn test_sqlite_integration() {
    use rustbasic_core::sql::driver::sqlite::SqliteConnection;
    use rustbasic_core::sql_params;

    let mut conn = SqliteConnection::connect(":memory:").unwrap();
    
    conn.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, age INTEGER)",
        &[]
    ).unwrap();

    let res = conn.execute(
        "INSERT INTO users (name, age) VALUES (?, ?)",
        &sql_params!["Alice", 30]
    ).unwrap();
    assert_eq!(res.rows_affected, 1);
    assert!(res.last_insert_id > 0);

    let rows = conn.query("SELECT id, name, age FROM users WHERE age = ?", &sql_params![30]).unwrap();
    assert_eq!(rows.len(), 1);
    let name: String = rows[0].get("name");
    let age: i64 = rows[0].get("age");
    assert_eq!(name, "Alice");
    assert_eq!(age, 30);
}

#[cfg(feature = "mysql")]
#[test]
fn test_mysql_connection_real() {
    use rustbasic_core::sql::driver::mysql::MySqlPool;
    use rustbasic_core::sql::driver::SqlConnection;
    use rustbasic_core::sql_params;

    // Connect to standard 'mysql' schema first to ensure test_rust exists
    let setup_pool = MySqlPool::new("127.0.0.1", 3306, "root", "1234", "mysql");
    let mut setup_conn = setup_pool.acquire().expect("Failed to acquire setup connection");
    
    setup_conn.execute("CREATE DATABASE IF NOT EXISTS test_rust", &[]).unwrap();

    // Connect to test_rust database
    let pool = MySqlPool::new("127.0.0.1", 3306, "root", "1234", "test_rust");
    let mut conn = pool.acquire().unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS test_users (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(100), age INT)",
        &[]
    ).unwrap();

    conn.execute("TRUNCATE TABLE test_users", &[]).unwrap();

    let res = conn.execute(
        "INSERT INTO test_users (name, age) VALUES (?, ?)",
        &sql_params!["Bob", 25]
    ).unwrap();
    assert_eq!(res.rows_affected, 1);

    let rows = conn.query("SELECT name, age FROM test_users", &[]).unwrap();
    assert_eq!(rows.len(), 1);
    let name: String = rows[0].get("name");
    let age: i32 = rows[0].get("age");
    assert_eq!(name, "Bob");
    assert_eq!(age, 25);
}

#[test]
fn test_unified_connection_url_and_macro() {
    use rustbasic_core::sql::driver::connect;
    use rustbasic_core::sql_params;

    #[cfg(feature = "sqlite")]
    {
        let mut conn = connect("sqlite://:memory:").unwrap();
        conn.execute("CREATE TABLE test_params (id INTEGER PRIMARY KEY, name TEXT)", &[]).unwrap();
        conn.execute("INSERT INTO test_params (name) VALUES (?)", &sql_params!["A"]).unwrap();
        let rows = conn.query("SELECT name FROM test_params", &[]).unwrap();
        assert_eq!(rows.len(), 1);
        let name: String = rows[0].get("name");
        assert_eq!(name, "A");
    }
}