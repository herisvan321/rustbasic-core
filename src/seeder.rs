use sea_orm::DatabaseConnection;

#[async_trait::async_trait]
pub trait SeederTrait {
    async fn run(&self, db: &DatabaseConnection) -> Result<(), sea_orm::DbErr>;
}
