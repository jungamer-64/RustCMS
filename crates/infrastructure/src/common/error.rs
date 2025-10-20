use thiserror::Error;

/// Errors that can occur within the infrastructure layer.
#[derive(Debug, Error, Clone)]
pub enum InfrastructureError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Search index error: {0}")]
    SearchError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Unknown infrastructure error: {0}")]
    Unknown(String),
}

pub type InfrastructureResult<T> = Result<T, InfrastructureError>;

pub type Result<T> = InfrastructureResult<T>;

#[cfg(feature = "database")]
impl From<diesel::result::Error> for InfrastructureError {
    fn from(err: diesel::result::Error) -> Self {
        use diesel::result::Error as DieselError;
        match err {
            DieselError::NotFound => InfrastructureError::DatabaseError("Record not found".into()),
            DieselError::DatabaseError(kind, info) => {
                InfrastructureError::DatabaseError(format!("Database error ({kind:?}): {info:?}"))
            }
            DieselError::QueryBuilderError(msg) => {
                InfrastructureError::DatabaseError(format!("Query builder error: {msg}"))
            }
            DieselError::DeserializationError(e) => {
                InfrastructureError::DatabaseError(format!("Deserialization error: {e}"))
            }
            DieselError::SerializationError(e) => {
                InfrastructureError::DatabaseError(format!("Serialization error: {e}"))
            }
            _ => InfrastructureError::DatabaseError(err.to_string()),
        }
    }
}

#[cfg(feature = "database")]
impl From<diesel::r2d2::PoolError> for InfrastructureError {
    fn from(err: diesel::r2d2::PoolError) -> Self {
        InfrastructureError::ConnectionError(format!("Connection pool error: {err}"))
    }
}
