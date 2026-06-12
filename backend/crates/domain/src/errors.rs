use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid email: {0}")]
    InvalidEmail(String),
    #[error("invalid name: {0}")]
    InvalidName(String),
    #[error("weak password: {0}")]
    WeakPassword(String),
    #[error("invalid task title: {0}")]
    InvalidTaskTitle(String),
    #[error("invalid task description: {0}")]
    InvalidTaskDescription(String),
    #[error("invalid task status: {0}")]
    InvalidTaskStatus(String),
    #[error("invalid task priority: {0}")]
    InvalidTaskPriority(String),
}
