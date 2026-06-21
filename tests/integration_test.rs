use rustbasic_core::sql::any::{AnyPool, Executor};
use rustbasic_core::sql_params;

#[cfg(feature = "sqlite")]
#[tokio::test]
async fn test_sqlite_integration() {
    let pool = AnyPool::connect("sqlite::memory:").await.unwrap();
    
    pool.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, age INTEGER)",
        &[]
    ).await.unwrap();

    let res = pool.execute(
        "INSERT INTO users (name, age) VALUES (?, ?)",
        &sql_params!["Alice", 30]
    ).await.unwrap();
    assert_eq!(res.rows_affected(), 1);
    assert!(res.last_insert_id().is_some());

    let rows = pool.fetch_all("SELECT id, name, age FROM users WHERE age = ?", &sql_params![30]).await.unwrap();
    assert_eq!(rows.len(), 1);
    let name: String = rows[0].get("name");
    let age: i64 = rows[0].get("age");
    assert_eq!(name, "Alice");
    assert_eq!(age, 30);
}

#[cfg(feature = "mysql")]
#[tokio::test]
async fn test_mysql_connection_real() {
    // Connect to standard 'mysql' schema first to ensure test_rust exists
    let setup_pool = AnyPool::connect("mysql://root:1234@127.0.0.1:3306/mysql").await.expect("Failed to connect to mysql");
    
    setup_pool.execute("CREATE DATABASE IF NOT EXISTS test_rust", &[]).await.unwrap();

    // Connect to test_rust database
    let pool = AnyPool::connect("mysql://root:1234@127.0.0.1:3306/test_rust").await.unwrap();

    pool.execute(
        "CREATE TABLE IF NOT EXISTS test_users (id INT AUTO_INCREMENT PRIMARY KEY, name VARCHAR(100), age INT)",
        &[]
    ).await.unwrap();

    pool.execute("TRUNCATE TABLE test_users", &[]).await.unwrap();

    let res = pool.execute(
        "INSERT INTO test_users (name, age) VALUES (?, ?)",
        &sql_params!["Bob", 25]
    ).await.unwrap();
    assert_eq!(res.rows_affected(), 1);

    let rows = pool.fetch_all("SELECT name, age FROM test_users", &[]).await.unwrap();
    assert_eq!(rows.len(), 1);
    let name: String = rows[0].get("name");
    let age: i32 = rows[0].get("age");
    assert_eq!(name, "Bob");
    assert_eq!(age, 25);
}

#[tokio::test]
async fn test_unified_connection_url_and_macro() {
    #[cfg(feature = "sqlite")]
    {
        let pool = AnyPool::connect("sqlite::memory:").await.unwrap();
        pool.execute("CREATE TABLE test_params (id INTEGER PRIMARY KEY, name TEXT)", &[]).await.unwrap();
        pool.execute("INSERT INTO test_params (name) VALUES (?)", &sql_params!["A"]).await.unwrap();
        let rows = pool.fetch_all("SELECT name FROM test_params", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);
        let name: String = rows[0].get("name");
        assert_eq!(name, "A");
    }
}
