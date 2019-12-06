use thiserror::Error;
use diesel;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("failed to connect to database: {0}")]
    ConnectionError(#[from] diesel::ConnectionError),
    #[error("diesel error while querying: {0}")]
    DieselError(#[from] diesel::result::Error),
}

pub type Result<T> = std::result::Result<T, DatabaseError>;
