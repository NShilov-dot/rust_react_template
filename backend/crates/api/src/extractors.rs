use axum::{extract::FromRequestParts, http::request::Parts};

use domain::UserId;

use crate::{error::ApiError, state::AppState};

/// Zero-cost extractor that rejects requests carrying `Sec-Fetch-Site: cross-site`.
///
/// Add as a parameter to any handler that authenticates via cookie (i.e. does NOT
/// require `Authorization: Bearer`). The check is defence-in-depth on top of the
/// `SameSite=Lax` cookie policy: Lax already prevents the cookie from being sent
/// on cross-site POSTs, but an explicit gate here covers:
/// - browsers that pre-date Fetch Metadata (`Sec-Fetch-Site` absent → we allow)
/// - future cookie policy relaxation by accident or misconfiguration
///
/// We allow `same-site` (subdomain requests) deliberately — tightening to
/// `same-origin`-only would break legitimate subdomain setups.
/// Direct/non-browser requests (curl, server-to-server) don't send `Sec-Fetch-Site`
/// and are allowed; they can't carry the HttpOnly cookie cross-site anyway.
pub struct SameSiteRequest;

impl<S: Send + Sync> FromRequestParts<S> for SameSiteRequest {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let site = parts
            .headers
            .get("sec-fetch-site")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("none");

        if site == "cross-site" {
            tracing::warn!(
                path = %parts.uri.path(),
                "CSRF: rejected cross-site request on cookie-authed endpoint",
            );
            return Err(ApiError::Forbidden);
        }

        Ok(SameSiteRequest)
    }
}

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
