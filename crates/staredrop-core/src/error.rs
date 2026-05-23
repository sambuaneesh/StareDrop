use thiserror::Error;

pub type Result<T> = std::result::Result<T, StareDropError>;

#[derive(Debug, Error)]
pub enum StareDropError {
    #[error("invalid data: {0}")]
    InvalidData(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("operation failed: {0}")]
    Failed(String),
}
