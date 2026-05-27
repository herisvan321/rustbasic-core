use sqlx::{AnyPool, Row};

pub type DbErr = sqlx::Error;

pub struct SchemaManager<'a> {
    pub pool: &'a AnyPool,
}

impl<'a> SchemaManager<'a> {
    pub fn new(pool: &'a AnyPool) -> Self {
        Self { pool }
    }
}

pub struct Schema;

impl Schema {
    pub async fn create<F>(manager: &SchemaManager<'_>, table_name: &str, callback: F) -> Result<(), DbErr>
    where
        F: FnOnce(&mut Blueprint),
    {
        let mut blueprint = Blueprint::new(table_name);
        callback(&mut blueprint);

        let sqls = blueprint.to_create_sqls(manager.pool).await;
        for sql in sqls {
            sqlx::query::<sqlx::Any>(&sql).execute(manager.pool).await?;
        }
        Ok(())
    }

    pub async fn table<F>(manager: &SchemaManager<'_>, table_name: &str, callback: F) -> Result<(), DbErr>
    where
        F: FnOnce(&mut Blueprint),
    {
        let mut blueprint = Blueprint::new(table_name);
        blueprint.auto_id = false;
        blueprint.timestamps = false;
        callback(&mut blueprint);

        let sqls = blueprint.to_alter_sqls(manager.pool).await;
        for sql in sqls {
            sqlx::query::<sqlx::Any>(&sql).execute(manager.pool).await?;
        }
        Ok(())
    }

    pub async fn drop(manager: &SchemaManager<'_>, table_name: &str) -> Result<(), DbErr> {
        let sql = format!("DROP TABLE IF EXISTS `{}`", table_name);
        sqlx::query(&sql).execute(manager.pool).await?;
        Ok(())
    }
}

pub struct Column {
    pub name: String,
    pub col_type: String,
    pub nullable: bool,
    pub unique: bool,
    pub primary_key: bool,
    pub default_val: Option<String>,
    pub is_indexed: bool,
}

pub struct ForeignKey {
    pub from_col: String,
    pub to_col: String,
    pub to_table: String,
    pub on_delete: Option<String>,
    pub on_update: Option<String>,
}

pub struct Blueprint {
    pub table_name: String,
    pub columns: Vec<Column>,
    pub foreign_keys: Vec<ForeignKey>,
    pub drop_columns: Vec<String>,
    pub auto_id: bool,
    pub timestamps: bool,
}

impl Blueprint {
    pub fn new(table_name: &str) -> Self {
        Self {
            table_name: table_name.to_string(),
            columns: Vec::new(),
            foreign_keys: Vec::new(),
            drop_columns: Vec::new(),
            auto_id: true,
            timestamps: true,
        }
    }

    pub fn no_id(&mut self) -> &mut Self {
        self.auto_id = false;
        self
    }

    pub fn no_timestamps(&mut self) -> &mut Self {
        self.timestamps = false;
        self
    }

    pub fn id(&mut self) -> &mut Self {
        self.auto_id = true;
        self
    }

    fn add_col(&mut self, name: &str, col_type: &str) -> &mut Column {
        self.columns.push(Column {
            name: name.to_string(),
            col_type: col_type.to_string(),
            nullable: false,
            unique: false,
            primary_key: false,
            default_val: None,
            is_indexed: false,
        });
        self.columns.last_mut().unwrap()
    }

    pub fn string(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "VARCHAR(255)");
        ColumnBuilder::new(self)
    }

    pub fn text(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "TEXT");
        ColumnBuilder::new(self)
    }

    pub fn integer(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "INTEGER");
        ColumnBuilder::new(self)
    }

    pub fn big_integer(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "BIGINT");
        ColumnBuilder::new(self)
    }

    pub fn float(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "FLOAT");
        ColumnBuilder::new(self)
    }

    pub fn double(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "DOUBLE");
        ColumnBuilder::new(self)
    }

    pub fn decimal(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "DECIMAL(10,2)");
        ColumnBuilder::new(self)
    }

    pub fn char(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "CHAR(255)");
        ColumnBuilder::new(self)
    }

    pub fn boolean(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "BOOLEAN");
        ColumnBuilder::new(self)
    }

    pub fn date_time(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "DATETIME");
        ColumnBuilder::new(self)
    }

    pub fn timestamp(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "TIMESTAMP");
        ColumnBuilder::new(self)
    }

    pub fn uuid(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "VARCHAR(36)");
        ColumnBuilder::new(self)
    }

    pub fn json(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "TEXT");
        ColumnBuilder::new(self)
    }

    pub fn json_binary(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "TEXT");
        ColumnBuilder::new(self)
    }

    pub fn binary(&mut self, name: &str) -> ColumnBuilder<'_> {
        self.add_col(name, "BLOB");
        ColumnBuilder::new(self)
    }

    pub fn timestamps(&mut self) -> &mut Self {
        self.timestamps = true;
        self
    }

    pub fn foreign<'a>(&'a mut self, from_col: &str) -> ForeignKeyBuilder<'a> {
        ForeignKeyBuilder::new(self, from_col)
    }

    pub fn drop_column(&mut self, name: &str) -> &mut Self {
        self.drop_columns.push(name.to_string());
        self
    }

    async fn to_alter_sqls(&self, pool: &AnyPool) -> Vec<String> {
        let mut sqls = Vec::new();
        let is_mysql = if let Ok(conn) = pool.acquire().await {
            conn.backend_name() == "MySQL"
        } else {
            false
        };

        // 1. Tambah kolom baru
        for col in &self.columns {
            let mut col_type = col.col_type.clone();
            if !is_mysql && (col_type == "DATETIME" || col_type == "TIMESTAMP") {
                col_type = "TEXT".to_string();
            }
            let mut col_def = format!("`{}` {}", col.name, col_type);
            if !col.nullable {
                col_def.push_str(" NOT NULL");
            }
            if col.unique {
                col_def.push_str(" UNIQUE");
            }
            if let Some(ref d) = col.default_val {
                col_def.push_str(&format!(" DEFAULT {}", d));
            }
            
            let sql = format!("ALTER TABLE `{}` ADD COLUMN {}", self.table_name, col_def);
            sqls.push(sql);
        }

        // 2. Hapus kolom (ALTER TABLE DROP COLUMN)
        for col_name in &self.drop_columns {
            let sql = format!("ALTER TABLE `{}` DROP COLUMN `{}`", self.table_name, col_name);
            sqls.push(sql);
        }

        sqls
    }

    async fn to_create_sqls(&self, pool: &AnyPool) -> Vec<String> {
        let mut sqls = Vec::new();
        let is_mysql = if let Ok(conn) = pool.acquire().await {
            conn.backend_name() == "MySQL"
        } else {
            false
        };

        let mut create_table = format!("CREATE TABLE IF NOT EXISTS `{}` (\n", self.table_name);
        let mut col_parts = Vec::new();

        if self.auto_id {
            if is_mysql {
                col_parts.push("`id` INT AUTO_INCREMENT PRIMARY KEY".to_string());
            } else {
                col_parts.push("`id` INTEGER PRIMARY KEY AUTOINCREMENT".to_string());
            }
        }

        for col in &self.columns {
            let mut col_type = col.col_type.clone();
            if !is_mysql && (col_type == "DATETIME" || col_type == "TIMESTAMP") {
                col_type = "TEXT".to_string();
            }
            let mut col_def = format!("`{}` {}", col.name, col_type);
            if col.primary_key && !self.auto_id {
                col_def.push_str(" PRIMARY KEY");
            }
            if !col.nullable {
                col_def.push_str(" NOT NULL");
            }
            if col.unique {
                col_def.push_str(" UNIQUE");
            }
            if let Some(ref d) = col.default_val {
                col_def.push_str(&format!(" DEFAULT {}", d));
            }
            col_parts.push(col_def);
        }

        if self.timestamps {
            if is_mysql {
                col_parts.push("`created_at` DATETIME DEFAULT CURRENT_TIMESTAMP".to_string());
                col_parts.push("`updated_at` DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string());
            } else {
                col_parts.push("`created_at` TEXT DEFAULT CURRENT_TIMESTAMP".to_string());
                col_parts.push("`updated_at` TEXT DEFAULT CURRENT_TIMESTAMP".to_string());
            }
        }

        for fk in &self.foreign_keys {
            let mut fk_def = format!(
                "FOREIGN KEY (`{}`) REFERENCES `{}` (`{}`)",
                fk.from_col, fk.to_table, fk.to_col
            );
            if let Some(ref del) = fk.on_delete {
                fk_def.push_str(&format!(" ON DELETE {}", del));
            }
            if let Some(ref upd) = fk.on_update {
                fk_def.push_str(&format!(" ON UPDATE {}", upd));
            }
            col_parts.push(fk_def);
        }

        create_table.push_str(&col_parts.join(",\n"));
        create_table.push_str("\n)");
        sqls.push(create_table);

        // Add indices
        for col in &self.columns {
            if col.is_indexed {
                sqls.push(format!(
                    "CREATE INDEX IF NOT EXISTS `{}_{}_idx` ON `{}` (`{}`)",
                    self.table_name, col.name, self.table_name, col.name
                ));
            }
        }

        sqls
    }
}

pub struct ColumnBuilder<'a> {
    blueprint: &'a mut Blueprint,
}

impl<'a> ColumnBuilder<'a> {
    pub fn new(blueprint: &'a mut Blueprint) -> Self {
        Self { blueprint }
    }

    fn current(&mut self) -> &mut Column {
        self.blueprint.columns.last_mut().unwrap()
    }

    pub fn not_null(mut self) -> Self {
        self.current().nullable = false;
        self
    }

    pub fn nullable(mut self) -> Self {
        self.current().nullable = true;
        self
    }

    pub fn unique(mut self) -> Self {
        self.current().unique = true;
        self
    }

    pub fn primary_key(mut self) -> Self {
        self.current().primary_key = true;
        self
    }

    pub fn default<T: ToString>(mut self, value: T) -> Self {
        self.current().default_val = Some(value.to_string());
        self
    }

    pub fn index(mut self) -> Self {
        self.current().is_indexed = true;
        self
    }
}

pub struct ForeignKeyBuilder<'a> {
    blueprint: &'a mut Blueprint,
    from_col: String,
    to_col: Option<String>,
    to_table: Option<String>,
    on_delete: Option<String>,
    on_update: Option<String>,
}

impl<'a> ForeignKeyBuilder<'a> {
    pub fn new(blueprint: &'a mut Blueprint, from_col: &str) -> Self {
        Self {
            blueprint,
            from_col: from_col.to_string(),
            to_col: None,
            to_table: None,
            on_delete: None,
            on_update: None,
        }
    }

    pub fn references(mut self, to_col: &str) -> Self {
        self.to_col = Some(to_col.to_string());
        self
    }

    pub fn on(mut self, to_table: &str) -> Self {
        self.to_table = Some(to_table.to_string());
        self
    }

    pub fn on_delete(mut self, action: &str) -> Self {
        self.on_delete = Some(action.to_uppercase());
        self
    }

    pub fn on_update(mut self, action: &str) -> Self {
        self.on_update = Some(action.to_uppercase());
        self
    }
}

impl<'a> Drop for ForeignKeyBuilder<'a> {
    fn drop(&mut self) {
        if let (Some(to_table), Some(to_col)) = (&self.to_table, &self.to_col) {
            self.blueprint.foreign_keys.push(ForeignKey {
                from_col: self.from_col.clone(),
                to_col: to_col.clone(),
                to_table: to_table.clone(),
                on_delete: self.on_delete.clone(),
                on_update: self.on_update.clone(),
            });
        }
    }
}

// -------------------------------------------------------------
// 📑 Migration & Migrator Trait
// -------------------------------------------------------------

#[crate::async_trait]
pub trait MigrationTrait: Send + Sync {
    fn name(&self) -> &str;
    async fn up<'a>(&self, manager: &'a SchemaManager<'a>) -> Result<(), DbErr>;
    async fn down<'a>(&self, manager: &'a SchemaManager<'a>) -> Result<(), DbErr>;
}

#[crate::async_trait]
pub trait MigratorTrait {
    fn migrations() -> Vec<Box<dyn MigrationTrait>>;

    async fn up(pool: &AnyPool, _steps: Option<u32>) -> Result<(), DbErr> {
        let manager = SchemaManager::new(pool);
        
        // 1. Setup migration history table
        sqlx::query::<sqlx::Any>("CREATE TABLE IF NOT EXISTS migration_history (
            version VARCHAR(255) PRIMARY KEY,
            applied_at BIGINT NOT NULL
        )").execute(pool).await?;

        // 2. Fetch applied migrations
        let rows = sqlx::query::<sqlx::Any>("SELECT version FROM migration_history").fetch_all(pool).await?;
        let applied: std::collections::HashSet<String> = rows.into_iter()
            .map(|r| r.get::<String, _>("version"))
            .collect();

        // 3. Apply pending migrations
        for migration in Self::migrations() {
            let name = migration.name();
            if !applied.contains(name) {
                migration.up(&manager).await?;
                let now = chrono::Utc::now().timestamp();
                sqlx::query::<sqlx::Any>("INSERT INTO migration_history (version, applied_at) VALUES (?, ?)")
                    .bind(name)
                    .bind(now)
                    .execute(pool)
                    .await?;
                println!("✅ Migration applied: {}", name);
            }
        }

        Ok(())
    }

    async fn down(pool: &AnyPool, _steps: Option<u32>) -> Result<(), DbErr> {
        let manager = SchemaManager::new(pool);
        
        // Get the last applied migration
        let row_opt = sqlx::query::<sqlx::Any>("SELECT version FROM migration_history ORDER BY applied_at DESC LIMIT 1")
            .fetch_optional(pool)
            .await?;

        if let Some(row) = row_opt {
            let version = row.get::<String, _>("version");
            for migration in Self::migrations() {
                if migration.name() == version {
                    migration.down(&manager).await?;
                    sqlx::query::<sqlx::Any>("DELETE FROM migration_history WHERE version = ?")
                        .bind(&version)
                        .execute(pool)
                        .await?;
                    println!("⬅️ Rollback applied: {}", version);
                    break;
                }
            }
        } else {
            println!("ℹ️ No migrations found to rollback.");
        }

        Ok(())
    }

    async fn fresh(pool: &AnyPool) -> Result<(), DbErr> {
        let manager = SchemaManager::new(pool);
        
        // 1. Rollback all migrations in reverse order
        let applied_rows = sqlx::query::<sqlx::Any>("SELECT version FROM migration_history ORDER BY applied_at DESC")
            .fetch_all(pool)
            .await
            .unwrap_or_default();

        let migrations = Self::migrations();
        for row in applied_rows {
            let version = row.get::<String, _>("version");
            if let Some(migration) = migrations.iter().find(|m| m.name() == version) {
                let _ = migration.down(&manager).await;
            }
        }

        // 2. Drop migration history table
        let _ = sqlx::query::<sqlx::Any>("DROP TABLE IF EXISTS migration_history").execute(pool).await;

        // 3. Rerun migrations
        Self::up(pool, None).await?;
        Ok(())
    }
}
