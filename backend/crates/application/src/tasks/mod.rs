pub mod create_task;
pub mod delete_task;
pub mod get_task;
pub mod list_tasks;
pub mod update_task;

use thiserror::Error;

use domain::DomainError;

use crate::ports::RepoError;

#[derive(Debug, Error)]
pub enum TaskError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repo(#[from] RepoError),
}
