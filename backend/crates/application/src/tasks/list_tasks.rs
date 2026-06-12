use std::sync::Arc;

use domain::{Task, TaskStatus, UserId};

use crate::ports::{RepoError, TaskListFilter, TaskRepository};

pub struct ListTasks {
    repo: Arc<dyn TaskRepository>,
}

pub struct ListTasksInput {
    pub owner_id: UserId,
    pub status: Option<TaskStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl ListTasks {
    pub fn new(repo: Arc<dyn TaskRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: ListTasksInput) -> Result<Vec<Task>, RepoError> {
        let limit = input.limit.unwrap_or(50).clamp(1, 200);
        let offset = input.offset.unwrap_or(0).max(0);
        let filter = TaskListFilter { status: input.status, limit, offset };
        self.repo.list_for_owner(input.owner_id, filter).await
    }
}
