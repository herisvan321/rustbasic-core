use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use chrono::Local;
use colored::*;
use super::utils::{to_snake_case, to_pascal_case};

pub fn make_controller(name: &str) {
    let pascal_name = to_pascal_case(name).replace("Controller", "");
    let snake_name = to_snake_case(&pascal_name);
    let class_name = format!("{}Controller", pascal_name);
    let file_name = format!("{}_controller.rs", snake_name);
    let file_path = format!("src/app/http/controllers/{}", file_name);

    if std::path::Path::new(&file_path).exists() {
        println!("{} {} {}", "⚠️  Controller".yellow(), file_path.cyan(), "sudah ada.".yellow());
        return;
    }

    let template = format!(
r#"/* ---------------------------------------------------------
 * 📑 LABEL: {class_name} ({file_name})
 * --------------------------------------------------------- */

use crate::app::view;
use rustbasic_core::requests::Request;
use rustbasic_core::axum::response::IntoResponse;
use rustbasic_core::minijinja::context;

pub struct {class_name};

impl {class_name} {{
    pub async fn index(req: Request) -> impl IntoResponse {{
        view(&req, "{snake_name}.rb.html", context! {{
            title => "{class_name}"
        }})
    }}
}}
"#, class_name = class_name, file_name = file_name, snake_name = snake_name);

    fs::write(&file_path, template).expect("Gagal membuat file controller");
    println!("{} {}", "✅ Controller dibuat:".green(), file_path.cyan());

    update_controller_mod_rs(&file_name.replace(".rs", ""));
}

pub fn update_controller_mod_rs(mod_name: &str) {
    let mod_path = "src/app/http/controllers/mod.rs";
    let mut content = String::new();
    if let Ok(mut file) = fs::File::open(mod_path) {
        file.read_to_string(&mut content).ok();
    }

    let line = format!("pub mod {};", mod_name);
    if content.contains(&line) {
        return;
    }

    let mut file = OpenOptions::new()
        .append(true)
        .open(mod_path)
        .expect("Gagal membuka controllers/mod.rs");

    writeln!(file, "{}", line).ok();
    println!("{} {}", "📝".blue(), "controllers/mod.rs diperbarui.".dimmed());
}

pub fn make_middleware(name: &str) {
    let snake_name = to_snake_case(name).replace("_middleware", "");
    let fn_name = format!("{}_middleware", snake_name);
    let file_name = format!("{}.rs", snake_name);
    let file_path = format!("src/app/http/middleware/{}", file_name);

    if std::path::Path::new(&file_path).exists() {
        println!("{} {} {}", "⚠️  Middleware".yellow(), file_path.cyan(), "sudah ada.".yellow());
        return;
    }

    let template = format!(
r#"/* ---------------------------------------------------------
 * 📑 LABEL: {label} (middleware/{file_name})
 * --------------------------------------------------------- */

use rustbasic_core::axum::{{
    extract::Request,
    middleware::Next,
    response::Response,
}};

pub async fn {fn_name}(
    req: Request,
    next: Next,
) -> Response {{
    // Lakukan sesuatu sebelum request sampai ke controller
    
    let response = next.run(req).await;
    
    // Lakukan sesuatu setelah request selesai diproses
    
    response
}}
"#, label = name.to_uppercase(), file_name = file_name, fn_name = fn_name);

    fs::write(&file_path, template).expect("Gagal membuat file middleware");
    println!("{} {}", "✅ Middleware dibuat:".green(), file_path.cyan());

    update_middleware_mod_rs(&snake_name);
}

pub fn update_middleware_mod_rs(mod_name: &str) {
    let mod_path = "src/app/http/middleware/mod.rs";
    let mut content = String::new();
    if let Ok(mut file) = fs::File::open(mod_path) {
        file.read_to_string(&mut content).ok();
    }

    let line = format!("pub mod {};", mod_name);
    if content.contains(&line) {
        return;
    }

    let mut file = OpenOptions::new()
        .append(true)
        .open(mod_path)
        .expect("Gagal membuka middleware/mod.rs");

    writeln!(file, "{}", line).ok();
    println!("{} {}", "📝".blue(), "middleware/mod.rs diperbarui.".dimmed());
}

pub fn make_model(name: &str) {
    let snake_name = to_snake_case(name);
    let table_name = format!("{}s", snake_name);
    let file_path = format!("src/app/models/{}.rs", snake_name);

    if std::path::Path::new(&file_path).exists() {
        println!("{} {} {}", "⚠️  Model".yellow(), file_path.cyan(), "sudah ada.".yellow());
        return;
    }

    let template = format!(
r#"use rustbasic_core::sea_orm::entity::prelude::*;
use serde::{{Deserialize, Serialize}};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "{}")]
pub struct Model {{
    #[sea_orm(primary_key)]
    pub id: i32,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<DateTime>,
}}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {{}}

impl ActiveModelBehavior for ActiveModel {{}}
"#, table_name);

    fs::write(&file_path, template).expect("Gagal membuat file model");
    println!("{} {}", "✅ Model dibuat:".green(), file_path.cyan());

    update_mod_rs(&to_pascal_case(name), &snake_name);
}

pub fn update_mod_rs(class_name: &str, snake_name: &str) {
    let mod_path = "src/app/models/mod.rs";
    let mut content = String::new();
    if let Ok(mut file) = fs::File::open(mod_path) {
        file.read_to_string(&mut content).ok();
    }

    let mod_line = format!("pub mod {};", snake_name);
    if content.contains(&mod_line) {
        return;
    }

    let mut file = OpenOptions::new()
        .append(true)
        .open(mod_path)
        .expect("Gagal membuka models/mod.rs");

    writeln!(file, "{}", mod_line).ok();
    writeln!(file, "pub use {}::Entity as {};", snake_name, class_name).ok();
    
    println!("{} {}", "📝".blue(), "models/mod.rs diperbarui.".dimmed());
}

pub fn make_rust_migration(name: &str) {
    let snake_name = to_snake_case(name);
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let mod_name = format!("m{}_{}", timestamp, snake_name);
    let file_path = format!("database/migrations/{}.rs", mod_name);

    if std::path::Path::new(&file_path).exists() {
        println!("{} {} {}", "⚠️  Migration".yellow(), file_path.cyan(), "sudah ada.".yellow());
        return;
    }

    let pascal_name = to_pascal_case(name);
    let table_iden = format!("{}s", pascal_name);

    let template = format!(
r#"use sea_orm_migration::prelude::*;
use async_trait::async_trait;

#[derive(Iden)]
enum {table_iden} {{
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
}}

#[derive(Iden)]
pub struct Migration;

impl MigrationName for Migration {{
    fn name(&self) -> &str {{
        "{mod_name}"
    }}
}}

#[async_trait]
impl MigrationTrait for Migration {{
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {{
        manager
            .create_table(
                Table::create()
                    .table({table_iden}::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new({table_iden}::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new({table_iden}::CreatedAt)
                            .date_time()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new({table_iden}::UpdatedAt)
                            .date_time()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }}

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {{
        manager
            .drop_table(Table::drop().table({table_iden}::Table).to_owned())
            .await
    }}
}}
"#, table_iden = table_iden, mod_name = mod_name);

    fs::write(&file_path, template).expect("Gagal membuat file migration");
    println!("{} {}", "✅ Migration Rust dibuat:".green(), file_path.cyan());

    update_migration_mod_rs(&mod_name);
}

pub fn update_migration_mod_rs(mod_name: &str) {
    let mod_path = "database/migrations/mod.rs";
    let mut content = String::new();
    if let Ok(mut file) = fs::File::open(mod_path) {
        file.read_to_string(&mut content).ok();
    }

    // Tambahkan mod declaration
    if !content.contains(&format!("pub mod {};", mod_name)) {
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&format!("pub mod {};\n", mod_name));
    }

    // Tambahkan ke list migrations
    let search_pattern = "fn migrations() -> Vec<Box<dyn MigrationTrait>> {";
    if let Some(_pos) = content.find(search_pattern) {
        let insert_pos = content.find("        ]").unwrap_or(content.len());
        content.insert_str(insert_pos, &format!("            Box::new({}::Migration),\n", mod_name));
    }

    fs::write(mod_path, content).expect("Gagal memperbarui database/migrations/mod.rs");
    println!("{} {}", "📝".blue(), "database/migrations/mod.rs diperbarui.".dimmed());
}

pub fn make_seeder(name: &str) {
    let pascal_name = to_pascal_case(name).replace("Seeder", "");
    let snake_name = to_snake_case(&pascal_name);
    let class_name = format!("{}Seeder", pascal_name);
    let file_name = format!("{}_seeder.rs", snake_name);
    let file_path = format!("database/seeders/{}", file_name);

    if std::path::Path::new(&file_path).exists() {
        println!("{} {} {}", "⚠️  Seeder".yellow(), file_path.cyan(), "sudah ada.".yellow());
        return;
    }

    let template = format!(
r#"#[allow(unused_imports)]
use rustbasic_core::sea_orm::{{DatabaseConnection, Set, ActiveModelTrait}};
use rustbasic_core::colored::Colorize;
use rustbasic_core::seeder::SeederTrait;
// use crate::app::models::{snake_name}; // Sesuaikan dengan model Anda

pub struct {class_name};

#[async_trait::async_trait]
impl SeederTrait for {class_name} {{
    async fn run(&self, _db: &DatabaseConnection) -> Result<(), rustbasic_core::sea_orm::DbErr> {{
        println!("   {{}} Sedang memproses {class_name}...", "⏳".blue());
        
        // Contoh:
        /*
        let _ = {snake_name}::ActiveModel {{
            name: Set("Example Data".to_owned()),
            ..Default::default()
        }}.insert(_db).await?;
        */

        Ok(())
    }}
}}
"#, class_name = class_name, snake_name = snake_name);

    fs::write(&file_path, template).expect("Gagal membuat file seeder");
    println!("{} {}", "✅ Seeder dibuat:".green(), file_path.cyan());

    update_seeder_mod_rs(&class_name, &file_name.replace(".rs", ""));
}

pub fn update_seeder_mod_rs(class_name: &str, mod_name: &str) {
    // 1. Update database/seeders/mod.rs (mod declaration)
    let db_mod_path = "database/seeders/mod.rs";
    let mut db_content = fs::read_to_string(db_mod_path).expect("Gagal membaca seeders/mod.rs");
    let mod_line = format!("pub mod {};", mod_name);
    if !db_content.contains(&mod_line) {
        db_content.push_str(&format!("{}\n", mod_line));
        fs::write(db_mod_path, db_content).ok();
    }

    // 2. Update src/app/seeder.rs (registration)
    let config_path = "src/app/seeder.rs";
    let mut config_content = fs::read_to_string(config_path).expect("Gagal membaca src/app/seeder.rs");
    let search_pattern = "let seeders: Vec<Box<dyn SeederTrait>> = vec![";
    if let Some(pos) = config_content.find(search_pattern) {
        let insert_pos = pos + search_pattern.len();
        config_content.insert_str(insert_pos, &format!("\n        Box::new(seeders::{}::{}),", mod_name, class_name));
        fs::write(config_path, config_content).ok();
    }
    
    println!("{} {}", "📝".blue(), "Pengaturan seeder diperbarui.".dimmed());
}
