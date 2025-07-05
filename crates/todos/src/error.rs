use thiserror::Error;
#[derive(Error, Debug)]
pub enum TodoError {
    #[error("Database error: {0}")]
    DbError(#[from] sea_orm::DbErr),
    #[error("{0} not found")]
    NotFound(String),
    #[error("id not found")]
    IDNotFound,
}
