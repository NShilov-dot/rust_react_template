use std::sync::Arc;

use domain::{TaskId, UserId};

use crate::ports::{RepoError, TaskRepository};

pub struct DeleteTask {
    repo: Arc<dyn TaskRepository>,
}

impl DeleteTask {
    pub fn new(repo: Arc<dyn TaskRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, id: TaskId, owner_id: UserId) -> Result<(), RepoError> {
        self.repo.delete(id, owner_id).await
    }
}
