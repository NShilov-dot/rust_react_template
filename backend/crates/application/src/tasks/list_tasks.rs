use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use domain::{Task, TaskStatus, UserId};

use crate::ports::{RepoError, TaskListFilter, TaskRepository};

/// Default page size for `/tasks` list. The frontend asks for explicit
/// `limit`, but we cap server-side regardless to keep query latency
/// bounded.
const DEFAULT_PAGE_SIZE: i64 = 30;
const MAX_PAGE_SIZE: i64 = 100;

pub struct ListTasks {
    repo: Arc<dyn TaskRepository>,
}

pub struct ListTasksInput {
    pub owner_id: UserId,
    pub status: Option<TaskStatus>,
    pub cursor: Option<(DateTime<Utc>, Uuid)>,
    pub limit: Option<i64>,
}

pub struct ListTasksOutput {
    pub tasks: Vec<Task>,
    /// Cursor for the next page — `None` means this was the last page.
    /// Format: `(created_at, id)` of the last included task.
    pub next_cursor: Option<(DateTime<Utc>, Uuid)>,
}

impl ListTasks {
    pub fn new(repo: Arc<dyn TaskRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, input: ListTasksInput) -> Result<ListTasksOutput, RepoError> {
        let limit = input
            .limit
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .clamp(1, MAX_PAGE_SIZE);
        // Ask the repo for `limit + 1` rows — if we got the extra, there's
        // another page, and the `limit`-th row is the cursor for it.
        let filter = TaskListFilter {
            status: input.status,
            cursor: input.cursor,
            limit: limit + 1,
        };
        let mut tasks = self.repo.list_for_owner(input.owner_id, filter).await?;

        let next_cursor = if tasks.len() as i64 > limit {
            tasks.pop(); // drop the peek row
            tasks.last().map(|t| (t.created_at, t.id.0))
        } else {
            None
        };

        Ok(ListTasksOutput { tasks, next_cursor })
    }
}
