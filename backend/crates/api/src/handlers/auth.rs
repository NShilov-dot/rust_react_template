use axum::{extract::State, http::StatusCode, Json};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use application::auth::{
    login::LoginInput, logout::LogoutInput, refresh::RefreshInput, register::RegisterInput,
};
use application::ports::TokenPair;

use crate::{error::ApiError, extractors::AuthUser, handlers::users::UserResponse, state::AppState};

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

fn refresh_cookie<'a>(value: String, max_age_secs: i64) -> Cookie<'a> {
    // SameSite=Strict + HttpOnly + Secure is the OWASP-recommended baseline.
    // Browsers (Chrome 88+, Firefox) treat localhost as a "secure context" so
    // `Secure` works in HTTP dev too — no need to toggle by env.
    Cookie::build((REFRESH_COOKIE, value))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(cookie::time::Duration::seconds(max_age_secs))
        .build()
}

fn clear_refresh_cookie<'a>() -> Cookie<'a> {
    // Same attributes as the set cookie + Max-Age=0 → browser drops it.
    Cookie::build((REFRESH_COOKIE, ""))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(cookie::time::Duration::ZERO)
        .build()
}

fn ttl_to_secs(tp: &TokenPair) -> i64 {
    (tp.refresh_expires_at - Utc::now()).num_seconds().max(0)
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
        .await?;

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
        .await?;

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
    jar: CookieJar,
) -> Result<(CookieJar, Json<AccessResponse>), ApiError> {
    let presented = jar
        .get(REFRESH_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or(ApiError::Unauthorized)?;

    let tokens = state
        .refresh
        .execute(RefreshInput {
            refresh_token: presented,
        })
        .await?;

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

