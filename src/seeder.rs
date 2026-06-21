use crate::sql::AnyPool;
use crate::colored::Colorize;

#[crate::async_trait]
pub trait SeederTrait: Send + Sync {
    async fn run<'a>(&'a self, db: &'a crate::sql::AnyPool) -> Result<(), crate::sql::Error>;
}

pub async fn run_seeders(db: &AnyPool, seeders: Vec<Box<dyn SeederTrait + Send + Sync>>) {
    println!("\n{}", "🌱 Menjalankan Seeder Database...".blue().bold());
    
    for seeder in seeders {
        if let Err(e) = seeder.run(db).await {
            println!("{} {}", "❌ Gagal menjalankan seeder:".red(), e);
        }
    }
    
    println!("{}", "✅ Semua seeder selesai diproses!".green().bold());
}
