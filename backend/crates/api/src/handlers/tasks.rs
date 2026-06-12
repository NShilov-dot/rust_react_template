use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use application::tasks::create_task::CreateTaskInput;
use application::tasks::list_tasks::ListTasksInput;
use application::tasks::update_task::UpdateTaskInput;
use domain::{Task, TaskId, TaskPriority, TaskStatus};

use crate::{error::ApiError, extractors::AuthUser, state::AppState};

// ─── Wire format ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TaskResponse {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub due_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Task> for TaskResponse {
    fn from(t: Task) -> Self {
        Self {
            id: t.id.0,
            owner_id: t.owner_id.0,
            title: t.title.as_str().to_string(),
            description: t.description.map(|d| d.as_str().to_string()),
            status: t.status,
            priority: t.priority,
            due_date: t.due_date,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub priority: Option<TaskPriority>,
    #[serde(default)]
    pub due_date: Option<DateTime<Utc>>,
}

/// PATCH body. `None` (= field absent) means "no change". For `description`,
/// an empty string clears it.
#[derive(Debug, Default, Deserialize)]
pub struct UpdateTaskRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<TaskStatus>,
    #[serde(default)]
    pub priority: Option<TaskPriority>,
    #[serde(default)]
    pub due_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub status: Option<TaskStatus>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

// ─── Handlers ────────────────────────────────────────────────────────

pub async fn create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(body): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), ApiError> {
    let task = state
        .create_task
        .execute(CreateTaskInput {
            owner_id: user_id,
            title: body.title,
            description: body.description,
            priority: body.priority,
            due_date: body.due_date,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(task.into())))
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<TaskResponse>>, ApiError> {
    let tasks = state
        .list_tasks
        .execute(ListTasksInput {
            owner_id: user_id,
            status: q.status,
            limit: q.limit,
            offset: q.offset,
        })
        .await?;
    Ok(Json(tasks.into_iter().map(Into::into).collect()))
}

pub async fn get(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<TaskResponse>, ApiError> {
    let task = state.get_task.execute(TaskId(id), user_id).await?;
    Ok(Json(task.into()))
}

pub async fn update(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTaskRequest>,
) -> Result<Json<TaskResponse>, ApiError> {
    let task = state
        .update_task
        .execute(UpdateTaskInput {
            id: TaskId(id),
            owner_id: user_id,
            title: body.title,
            description: body.description,
            status: body.status,
            priority: body.priority,
            due_date: body.due_date,
        })
        .await?;
    Ok(Json(task.into()))
}

pub async fn delete(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state.delete_task.execute(TaskId(id), user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
