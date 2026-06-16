use axum::{extract::State, http::StatusCode, Json};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use application::auth::{
    login::LoginInput, logout::LogoutInput, refresh::RefreshInput, register::RegisterInput,
};
use application::ports::TokenPair;

use crate::{
    error::ApiError,
    extractors::{AuthUser, SameSiteRequest},
    handlers::users::UserResponse,
    state::AppState,
};

/// Cookie name carrying the refresh token. HttpOnly — JS can't read it.
const REFRESH_COOKIE: &str = "refresh_token";

// ─── Requests ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub name: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

// `/auth/refresh` and `/auth/logout` no longer need a JSON body — the refresh
// token lives in the HttpOnly cookie.

// ─── Responses (no refresh_token in JSON anymore) ────────────────────

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub access_expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AccessResponse {
    pub access_token: String,
    pub access_expires_at: DateTime<Utc>,
}

// ─── Cookie helpers ──────────────────────────────────────────────────

pub(crate) fn refresh_cookie<'a>(value: String, max_age_secs: i64) -> Cookie<'a> {
    // HttpOnly + Secure + SameSite=Lax. NOT Strict: the Google OAuth flow
    // ends with a cross-site top-level redirect that sets the cookie and
    // bounces to /dashboard; Safari and older Firefox drop Strict cookies
    // set during a cross-site redirect chain on the *first* follow-up
    // request — i.e. the SessionBootstrap refresh fires without the cookie
    // and the user appears logged out right after signing in. Lax allows
    // the cookie on top-level GETs (what OAuth needs) and still blocks
    // CSRF on the only state-changing cookie-authed endpoints we have
    // (/auth/refresh, /auth/logout) since browsers don't attach Lax
    // cookies on cross-site POSTs.
    // localhost counts as a "secure context" so `Secure` works in HTTP dev.
    Cookie::build((REFRESH_COOKIE, value))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::seconds(max_age_secs))
        .build()
}

pub(crate) fn clear_refresh_cookie<'a>() -> Cookie<'a> {
    // Same attributes as the set cookie + Max-Age=0 → browser drops it.
    Cookie::build((REFRESH_COOKIE, ""))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::ZERO)
        .build()
}

fn ttl_to_secs(tp: &TokenPair) -> i64 {
    (tp.refresh_expires_at - Utc::now()).num_seconds().max(0)
}

// ─── Metrics helper ──────────────────────────────────────────────────
//
// Single counter `auth_attempts_total` with two labels: `endpoint`
// (register|login|refresh|logout) and `outcome` (success|failure). That's
// at most 8 series total — safely below any Prom cardinality budget.
// We deliberately do NOT label by error variant or user-id (cardinality
// explosion risk).
fn record(endpoint: &'static str, outcome: &'static str) {
    metrics::counter!("auth_attempts_total", "endpoint" => endpoint, "outcome" => outcome)
        .increment(1);
}

// ─── Handlers ────────────────────────────────────────────────────────

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<RegisterRequest>,
) -> Result<(CookieJar, (StatusCode, Json<AuthResponse>)), ApiError> {
    let out = state
        .register
        .execute(RegisterInput {
            email: body.email,
            name: body.name,
            password: body.password,
        })
        .await
        .inspect_err(|_| record("register", "failure"))?;
    record("register", "success");

    let max_age = ttl_to_secs(&out.tokens);
    let jar = jar.add(refresh_cookie(out.tokens.refresh_token, max_age));

    Ok((
        jar,
        (
            StatusCode::CREATED,
            Json(AuthResponse {
                user: out.user.into(),
                access_token: out.tokens.access_token,
                access_expires_at: out.tokens.access_expires_at,
            }),
        ),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<LoginRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), ApiError> {
    let out = state
        .login
        .execute(LoginInput {
            email: body.email,
            password: body.password,
        })
        .await
        .inspect_err(|_| record("login", "failure"))?;
    record("login", "success");

    let max_age = ttl_to_secs(&out.tokens);
    let jar = jar.add(refresh_cookie(out.tokens.refresh_token, max_age));

    Ok((
        jar,
        Json(AuthResponse {
            user: out.user.into(),
            access_token: out.tokens.access_token,
            access_expires_at: out.tokens.access_expires_at,
        }),
    ))
}

pub async fn refresh(
    State(state): State<AppState>,
    _csrf: SameSiteRequest,
    jar: CookieJar,
) -> Result<(CookieJar, Json<AccessResponse>), ApiError> {
    let presented = jar
        .get(REFRESH_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or_else(|| {
            record("refresh", "failure");
            ApiError::Unauthorized
        })?;

    let tokens = state
        .refresh
        .execute(RefreshInput {
            refresh_token: presented,
        })
        .await
        .inspect_err(|_| record("refresh", "failure"))?;
    record("refresh", "success");

    let max_age = ttl_to_secs(&tokens);
    let jar = jar.add(refresh_cookie(tokens.refresh_token, max_age));

    Ok((
        jar,
        Json(AccessResponse {
            access_token: tokens.access_token,
            access_expires_at: tokens.access_expires_at,
        }),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    _csrf: SameSiteRequest,
    jar: CookieJar,
) -> Result<(CookieJar, StatusCode), ApiError> {
    if let Some(c) = jar.get(REFRESH_COOKIE) {
        // best-effort revoke; we clear the cookie either way
        let _ = state
            .logout
            .execute(LogoutInput {
                refresh_token: c.value().to_string(),
            })
            .await;
    }
    // Always count as success — logout is idempotent and we always clear
    // the cookie regardless of whether the revoke call worked.
    record("logout", "success");
    let jar = jar.add(clear_refresh_cookie());
    Ok((jar, StatusCode::NO_CONTENT))
}

pub async fn me(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
) -> Result<Json<UserResponse>, ApiError> {
    let user = state.get_user.execute(user_id).await?;
    Ok(Json(user.into()))
}
