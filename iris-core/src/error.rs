//! Error types for iris-core.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IrisError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("vault error: {0}")]
    Vault(String),
}

pub type IrisResult<T> = Result<T, IrisError>;
