use std::sync::Arc;

use chrono::{DateTime, Utc};

use domain::{Task, TaskDescription, TaskPriority, TaskTitle, UserId};

use crate::ports::TaskRepository;
use crate::tasks::TaskError;

#[derive(Debug, Clone)]
pub struct CreateTaskInput {
    pub owner_id: UserId,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<TaskPriority>,
    pub due_date: Option<DateTime<Utc>>,
}

pub struct CreateTask {
    repo: Arc<dyn TaskRepository>,
}

impl CreateTask {
    pub fn new(repo: Arc<dyn TaskRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: CreateTaskInput) -> Result<Task, TaskError> {
        let title = TaskTitle::parse(input.title)?;
        let description = match input.description {
            Some(raw) if !raw.trim().is_empty() => Some(TaskDescription::parse(raw)?),
            _ => None,
        };
        let task = Task::new(
            input.owner_id,
            title,
            description,
            input.priority.unwrap_or(TaskPriority::Medium),
            input.due_date,
        );
        self.repo.create(&task).await?;
        Ok(task)
    }
}
