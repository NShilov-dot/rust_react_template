use axum::{extract::FromRequestParts, http::request::Parts};

use domain::UserId;

use crate::{error::ApiError, state::AppState};

/// Extractor that validates the `Authorization: Bearer <jwt>` header and
/// returns the authenticated user's id. Reject with 401 on any failure.
pub struct AuthUser(pub UserId);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(ApiError::Unauthorized)?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(ApiError::Unauthorized)?
            .trim();

        let user_id = state
            .sessions
            .verify_access(token)
            .map_err(|_| ApiError::Unauthorized)?;

        Ok(AuthUser(user_id))
    }
}
