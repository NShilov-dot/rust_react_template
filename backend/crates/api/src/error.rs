use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

use application::auth::{login::LoginError, register::RegisterError};
use application::ports::{RepoError, SessionError};
use application::tasks::TaskError;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal error")]
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()),
            Self::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            Self::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            Self::Internal => {
                tracing::error!("internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string())
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

impl From<RepoError> for ApiError {
    fn from(e: RepoError) -> Self {
        match e {
            RepoError::NotFound => Self::NotFound,
            RepoError::Conflict(m) => Self::Conflict(m),
            RepoError::Storage(m) => {
                tracing::error!(error = %m, "storage error");
                Self::Internal
            }
        }
    }
}

impl From<SessionError> for ApiError {
    fn from(e: SessionError) -> Self {
        match e {
            SessionError::Invalid => Self::Unauthorized,
            SessionError::Backend(m) => {
                tracing::error!(error = %m, "session backend error");
                Self::Internal
            }
        }
    }
}

impl From<RegisterError> for ApiError {
    fn from(e: RegisterError) -> Self {
        match e {
            RegisterError::Domain(d) => Self::BadRequest(d.to_string()),
            RegisterError::EmailTaken => Self::Conflict("email already taken".into()),
            RegisterError::Repo(r) => r.into(),
            RegisterError::Hasher(_) => Self::Internal,
            RegisterError::Session(s) => s.into(),
        }
    }
}

impl From<LoginError> for ApiError {
    fn from(e: LoginError) -> Self {
        match e {
            LoginError::InvalidCredentials => Self::Unauthorized,
            LoginError::Repo(r) => r.into(),
            LoginError::Hasher(_) => Self::Internal,
            LoginError::Session(s) => s.into(),
        }
    }
}

impl From<TaskError> for ApiError {
    fn from(e: TaskError) -> Self {
        match e {
            TaskError::Domain(d) => Self::BadRequest(d.to_string()),
            TaskError::Repo(r) => r.into(),
        }
    }
}
