use std::sync::Arc;

use chrono::{DateTime, Utc};

use domain::{Task, TaskDescription, TaskId, TaskPriority, TaskStatus, TaskTitle, UserId};

use crate::ports::{RepoError, TaskRepository};
use crate::tasks::TaskError;

/// Partial update — `None` means "no change". For nullable fields:
/// - `description: Some("")` clears the description, `Some(text)` sets it.
/// - `due_date` cannot be cleared via PATCH in MVP; pick a new value or
///   leave the current one.
#[derive(Debug, Clone, Default)]
pub struct UpdateTaskInput {
    pub id: TaskId,
    pub owner_id: UserId,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub due_date: Option<DateTime<Utc>>,
}

pub struct UpdateTask {
    repo: Arc<dyn TaskRepository>,
}

impl UpdateTask {
    pub fn new(repo: Arc<dyn TaskRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: UpdateTaskInput) -> Result<Task, TaskError> {
        let mut task = self
            .repo
            .find_for_owner(input.id, input.owner_id)
            .await?
            .ok_or(RepoError::NotFound)?;

        if let Some(raw) = input.title {
            task.title = TaskTitle::parse(raw)?;
        }
        if let Some(raw) = input.description {
            task.description = if raw.trim().is_empty() {
                None
            } else {
                Some(TaskDescription::parse(raw)?)
            };
        }
        if let Some(status) = input.status {
            task.status = status;
        }
        if let Some(priority) = input.priority {
            task.priority = priority;
        }
        if let Some(due_date) = input.due_date {
            task.due_date = Some(due_date);
        }

        task.touch();
        self.repo.update(&task).await?;
        Ok(task)
    }
}

impl UpdateTaskInput {
    pub fn new(id: TaskId, owner_id: UserId) -> Self {
        Self {
            id,
            owner_id,
            title: None,
            description: None,
            status: None,
            priority: None,
            due_date: None,
        }
    }
}
