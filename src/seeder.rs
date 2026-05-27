#[crate::async_trait]
pub trait SeederTrait {
    async fn run<'a>(&'a self, db: &'a sqlx::AnyPool) -> Result<(), sqlx::Error>;
}
