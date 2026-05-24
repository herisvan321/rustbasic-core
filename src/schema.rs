use sea_orm_migration::prelude::*;

pub struct Schema;

impl Schema {
    pub async fn create<F>(manager: &SchemaManager<'_>, table_name: &str, callback: F) -> Result<(), DbErr>
    where
        F: FnOnce(&mut Blueprint),
    {
        let mut blueprint = Blueprint::new(table_name);
        blueprint.is_alter = false;
        callback(&mut blueprint);
        
        let mut table_stmt = blueprint.table;
        
        // 1. Prepend ID column if auto_id is enabled
        if blueprint.auto_id {
            table_stmt.col(
                ColumnDef::new(Alias::new("id"))
                    .integer()
                    .not_null()
                    .auto_increment()
                    .primary_key()
            );
        }
        
        // 2. Add user-defined columns
        for mut col in blueprint.columns {
            table_stmt.col(&mut col);
        }
        
        // 3. Append timestamps columns if timestamps is enabled
        if blueprint.timestamps {
            table_stmt.col(
                ColumnDef::new(Alias::new("created_at"))
                    .date_time()
                    .not_null()
                    .default(Expr::current_timestamp())
            );
            table_stmt.col(
                ColumnDef::new(Alias::new("updated_at"))
                    .date_time()
                    .not_null()
                    .default(Expr::current_timestamp())
            );
        }
        
        // 4. Create table
        manager.create_table(table_stmt).await?;
        
        // 5. Create indices
        for index in blueprint.indices {
            manager.create_index(index).await?;
        }
        
        Ok(())
    }

    pub async fn table<F>(manager: &SchemaManager<'_>, table_name: &str, callback: F) -> Result<(), DbErr>
    where
        F: FnOnce(&mut Blueprint),
    {
        let mut blueprint = Blueprint::new(table_name);
        blueprint.is_alter = true;
        blueprint.auto_id = false;      // Disable auto_id on alter by default
        blueprint.timestamps = false;   // Disable timestamps on alter by default
        callback(&mut blueprint);
        
        let mut alter_stmt = blueprint.alter;
        for mut col in blueprint.columns {
            alter_stmt.add_column(&mut col);
        }
        
        manager.alter_table(alter_stmt).await?;
        
        for index in blueprint.indices {
            manager.create_index(index).await?;
        }
        
        Ok(())
    }

    pub async fn drop(manager: &SchemaManager<'_>, table_name: &str) -> Result<(), DbErr> {
        manager.drop_table(
            Table::drop()
                .table(Alias::new(table_name))
                .to_owned()
        ).await
    }
}

pub struct Blueprint {
    pub table_name: String,
    pub table: TableCreateStatement,
    pub alter: TableAlterStatement,
    pub is_alter: bool,
    pub indices: Vec<IndexCreateStatement>,
    pub columns: Vec<ColumnDef>,
    pub auto_id: bool,
    pub timestamps: bool,
}

impl Blueprint {
    pub fn new(table_name: &str) -> Self {
        let mut table = Table::create();
        table.table(Alias::new(table_name)).if_not_exists();
        
        let mut alter = Table::alter();
        alter.table(Alias::new(table_name));
        
        Self {
            table_name: table_name.to_string(),
            table,
            alter,
            is_alter: false,
            indices: Vec::new(),
            columns: Vec::new(),
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

    pub fn string<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.string();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn text<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.text();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn integer<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.integer();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn big_integer<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.big_integer();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn float<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.float();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn double<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.double();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn decimal<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.decimal();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn char<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.char();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn boolean<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.boolean();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn date_time<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.date_time();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn timestamp<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.timestamp();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn uuid<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.uuid();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn json<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.json();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn json_binary<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.json_binary();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn binary<'a>(&'a mut self, name: &str) -> ColumnBuilder<'a> {
        let mut col_def = ColumnDef::new(Alias::new(name));
        col_def.binary();
        ColumnBuilder::new(self, name, col_def)
    }

    pub fn timestamps(&mut self) -> &mut Self {
        self.timestamps = true;
        self
    }

    pub fn drop_column(&mut self, name: &str) -> &mut Self {
        self.alter.drop_column(Alias::new(name));
        self
    }

    pub fn foreign<'a>(&'a mut self, from_col: &str) -> ForeignKeyBuilder<'a> {
        ForeignKeyBuilder::new(self, from_col)
    }
}

pub struct ColumnBuilder<'a> {
    blueprint: &'a mut Blueprint,
    col_name: String,
    col_def: Option<ColumnDef>,
    is_indexed: bool,
}

impl<'a> ColumnBuilder<'a> {
    pub fn new(blueprint: &'a mut Blueprint, col_name: &str, col_def: ColumnDef) -> Self {
        Self {
            blueprint,
            col_name: col_name.to_string(),
            col_def: Some(col_def),
            is_indexed: false,
        }
    }

    pub fn not_null(mut self) -> Self {
        if let Some(ref mut col) = self.col_def {
            col.not_null();
        }
        self
    }

    pub fn nullable(mut self) -> Self {
        if let Some(ref mut col) = self.col_def {
            col.null();
        }
        self
    }

    pub fn unique(mut self) -> Self {
        if let Some(ref mut col) = self.col_def {
            col.unique_key();
        }
        self
    }

    pub fn primary_key(mut self) -> Self {
        if let Some(ref mut col) = self.col_def {
            col.primary_key();
        }
        self
    }

    pub fn default<T>(mut self, value: T) -> Self 
    where
        T: Into<SimpleExpr>,
    {
        if let Some(ref mut col) = self.col_def {
            col.default(value);
        }
        self
    }

    pub fn index(mut self) -> Self {
        self.is_indexed = true;
        self
    }
}

impl<'a> Drop for ColumnBuilder<'a> {
    fn drop(&mut self) {
        if let Some(col) = self.col_def.take() {
            self.blueprint.columns.push(col);
            if self.is_indexed {
                let index_name = format!("{}_{}_index", self.blueprint.table_name, self.col_name);
                let index_stmt = Index::create()
                    .name(&index_name)
                    .table(Alias::new(&self.blueprint.table_name))
                    .col(Alias::new(&self.col_name))
                    .to_owned();
                self.blueprint.indices.push(index_stmt);
            }
        }
    }
}

pub struct ForeignKeyBuilder<'a> {
    blueprint: &'a mut Blueprint,
    from_col: String,
    to_col: Option<String>,
    to_table: Option<String>,
    on_delete: Option<ForeignKeyAction>,
    on_update: Option<ForeignKeyAction>,
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
        let act = match action.to_lowercase().as_str() {
            "cascade" => ForeignKeyAction::Cascade,
            "restrict" => ForeignKeyAction::Restrict,
            "set null" | "set_null" => ForeignKeyAction::SetNull,
            "no action" | "no_action" => ForeignKeyAction::NoAction,
            _ => ForeignKeyAction::Cascade,
        };
        self.on_delete = Some(act);
        self
    }

    pub fn on_update(mut self, action: &str) -> Self {
        let act = match action.to_lowercase().as_str() {
            "cascade" => ForeignKeyAction::Cascade,
            "restrict" => ForeignKeyAction::Restrict,
            "set null" | "set_null" => ForeignKeyAction::SetNull,
            "no action" | "no_action" => ForeignKeyAction::NoAction,
            _ => ForeignKeyAction::Cascade,
        };
        self.on_update = Some(act);
        self
    }
}

impl<'a> Drop for ForeignKeyBuilder<'a> {
    fn drop(&mut self) {
        if let (Some(to_table), Some(to_col)) = (&self.to_table, &self.to_col) {
            let mut fk = ForeignKey::create();
            
            let fk_name = format!(
                "fk_{}_{}_{}_{}",
                self.blueprint.table_name, self.from_col, to_table, to_col
            );
            
            fk.name(&fk_name)
              .from(Alias::new(&self.blueprint.table_name), Alias::new(&self.from_col))
              .to(Alias::new(to_table), Alias::new(to_col));
              
            if let Some(action) = self.on_delete {
                fk.on_delete(action);
            }
            if let Some(action) = self.on_update {
                fk.on_update(action);
            }
            
            self.blueprint.table.foreign_key(&mut fk);
        }
    }
}
