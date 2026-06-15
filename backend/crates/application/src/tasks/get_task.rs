use std::sync::Arc;

use domain::{Task, TaskId, UserId};

use crate::ports::{RepoError, TaskRepository};

pub struct GetTask {
    repo: Arc<dyn TaskRepository>,
}

impl GetTask {
    pub fn new(repo: Arc<dyn TaskRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, id: TaskId, owner_id: UserId) -> Result<Task, RepoError> {
        self.repo
            .find_for_owner(id, owner_id)
            .await?
            .ok_or(RepoError::NotFound)
    }
}
