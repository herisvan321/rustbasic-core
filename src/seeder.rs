

#[async_trait::async_trait]
pub trait SeederTrait {
    async fn run(&self, db: &crate::sea_orm::DatabaseConnection) -> Result<(), crate::sea_orm::DbErr>;
}
