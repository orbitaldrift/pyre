#![allow(dead_code)]

pub mod sync;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[async_trait::async_trait]
pub trait Dao: Sized {
    type Id;
    type Dal;

    async fn get(dal: Self::Dal, id: Self::Id) -> Result<Option<Self>, Error>;
    async fn delete(dal: Self::Dal, id: Self::Id) -> Result<(), Error>;

    async fn create(&mut self, dal: Self::Dal) -> Result<(), Error>;
    async fn update(&self, dal: Self::Dal) -> Result<Self::Id, Error>;
}
