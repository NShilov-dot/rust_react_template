use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use application::ports::{RepoError, TaskListFilter, TaskRepository};
use domain::{
    Task, TaskDescription, TaskId, TaskPriority, TaskStatus, TaskTitle, UserId,
};

pub struct PgTaskRepository {
    pool: PgPool,
}

impl PgTaskRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: Uuid,
    owner_id: Uuid,
    title: String,
    description: Option<String>,
    status: String,
    priority: String,
    due_date: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<TaskRow> for Task {
    type Error = RepoError;

    fn try_from(row: TaskRow) -> Result<Self, Self::Error> {
        // Any failure here means the DB has a value that the domain can no
        // longer represent — never a 400 to the client, always a 500.
        let title = TaskTitle::parse(row.title)
            .map_err(|e| RepoError::Storage(format!("invalid task title in db: {e}")))?;
        let description = match row.description {
            Some(raw) if !raw.is_empty() => Some(
                TaskDescription::parse(raw)
                    .map_err(|e| RepoError::Storage(format!("invalid task description in db: {e}")))?,
            ),
            _ => None,
        };
        let status = TaskStatus::parse(&row.status)
            .map_err(|e| RepoError::Storage(format!("invalid status in db: {e}")))?;
        let priority = TaskPriority::parse(&row.priority)
            .map_err(|e| RepoError::Storage(format!("invalid priority in db: {e}")))?;

        Ok(Task {
            id: TaskId(row.id),
            owner_id: UserId(row.owner_id),
            title,
            description,
            status,
            priority,
            due_date: row.due_date,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn map_sqlx(e: sqlx::Error) -> RepoError {
    match e {
        sqlx::Error::RowNotFound => RepoError::NotFound,
        other => RepoError::Storage(other.to_string()),
    }
}

#[async_trait]
impl TaskRepository for PgTaskRepository {
    async fn create(&self, task: &Task) -> Result<(), RepoError> {
        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, owner_id, title, description, status, priority,
                due_date, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(task.id.0)
        .bind(task.owner_id.0)
        .bind(task.title.as_str())
        .bind(task.description.as_ref().map(|d| d.as_str()))
        .bind(task.status.as_str())
        .bind(task.priority.as_str())
        .bind(task.due_date)
        .bind(task.created_at)
        .bind(task.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;
        Ok(())
    }

    async fn find_for_owner(
        &self,
        id: TaskId,
        owner_id: UserId,
    ) -> Result<Option<Task>, RepoError> {
        let row: Option<TaskRow> = sqlx::query_as(
            r#"
            SELECT id, owner_id, title, description, status, priority,
                   due_date, created_at, updated_at
            FROM tasks
            WHERE id = $1 AND owner_id = $2
            "#,
        )
        .bind(id.0)
        .bind(owner_id.0)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;
        row.map(Task::try_from).transpose()
    }

    async fn list_for_owner(
        &self,
        owner_id: UserId,
        filter: TaskListFilter,
    ) -> Result<Vec<Task>, RepoError> {
        // One query per branch so Postgres can pick a planner that uses the
        // composite index on (owner_id, status, created_at DESC) either way.
        let rows: Vec<TaskRow> = if let Some(status) = filter.status {
            sqlx::query_as(
                r#"
                SELECT id, owner_id, title, description, status, priority,
                       due_date, created_at, updated_at
                FROM tasks
                WHERE owner_id = $1 AND status = $2
                ORDER BY created_at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(owner_id.0)
            .bind(status.as_str())
            .bind(filter.limit)
            .bind(filter.offset)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as(
                r#"
                SELECT id, owner_id, title, description, status, priority,
                       due_date, created_at, updated_at
                FROM tasks
                WHERE owner_id = $1
                ORDER BY created_at DESC
                LIMIT $2 OFFSET $3
                "#,
            )
            .bind(owner_id.0)
            .bind(filter.limit)
            .bind(filter.offset)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(map_sqlx)?;
        rows.into_iter().map(Task::try_from).collect()
    }

    async fn update(&self, task: &Task) -> Result<(), RepoError> {
        // owner_id is part of the WHERE clause as a belt-and-braces check —
        // even though `find_for_owner` was called first, an attacker who
        // somehow forged a task body referencing a different owner can't
        // overwrite a row they don't own.
        let res = sqlx::query(
            r#"
            UPDATE tasks
            SET title = $3,
                description = $4,
                status = $5,
                priority = $6,
                due_date = $7,
                updated_at = $8
            WHERE id = $1 AND owner_id = $2
            "#,
        )
        .bind(task.id.0)
        .bind(task.owner_id.0)
        .bind(task.title.as_str())
        .bind(task.description.as_ref().map(|d| d.as_str()))
        .bind(task.status.as_str())
        .bind(task.priority.as_str())
        .bind(task.due_date)
        .bind(task.updated_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;

        if res.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }

    async fn delete(&self, id: TaskId, owner_id: UserId) -> Result<(), RepoError> {
        let res = sqlx::query(
            r#"
            DELETE FROM tasks
            WHERE id = $1 AND owner_id = $2
            "#,
        )
        .bind(id.0)
        .bind(owner_id.0)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;

        if res.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }
}
