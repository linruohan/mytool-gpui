use thiserror::Error;
#[derive(Error, Debug)]
pub enum TodoError {
    #[error("Database error: {0}")]
    DbError(#[from] sea_orm::DbErr),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("{0} not found")]
    NotFound(String),
    #[error("id not found")]
    IDNotFound,
    #[error("{0} already exists")]
    AlreadyExists(String),
    #[error("Operation timeout: {0}")]
    Timeout(String),
}
