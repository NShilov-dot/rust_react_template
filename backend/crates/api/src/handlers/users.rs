use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use domain::{User, UserId};

use crate::{error::ApiError, extractors::AuthUser, state::AppState};

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id.0,
            email: u.email.as_str().to_string(),
            name: u.name,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

pub async fn get(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, ApiError> {
    let user = state.get_user.execute(UserId(id)).await?;
    Ok(Json(user.into()))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    _auth: AuthUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<UserResponse>>, ApiError> {
    let users = state
        .list_users
        .execute(q.limit.unwrap_or(20), q.offset.unwrap_or(0))
        .await?;
    Ok(Json(users.into_iter().map(Into::into).collect()))
}
