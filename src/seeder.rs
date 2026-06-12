#[crate::async_trait]
pub trait SeederTrait {
    async fn run<'a>(&'a self, db: &'a crate::sql::AnyPool) -> Result<(), crate::sql::Error>;
}
