use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid email: {0}")]
    InvalidEmail(String),
    #[error("invalid name: {0}")]
    InvalidName(String),
    #[error("weak password: {0}")]
    WeakPassword(String),
}
