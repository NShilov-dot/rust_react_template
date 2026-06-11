use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use chrono::Utc;
use serde::Deserialize;
use tracing::warn;

use application::auth::google::GoogleAuthError;
use application::ports::OAuthError;

use crate::handlers::auth::{clear_refresh_cookie, refresh_cookie};
use crate::state::AppState;

/// `GET /auth/google/start` — kicks off the Authorization Code flow:
/// builds the authorize URL (with PKCE + state), persists the verifier
/// in the cache store, and 302s the browser to Google.
pub async fn start(State(state): State<AppState>) -> Response {
    let Some(google) = state.google_auth.as_ref() else {
        return feature_disabled();
    };

    match google.start().await {
        Ok(url) => Redirect::to(&url).into_response(),
        Err(e) => {
            warn!(error = %e, "google /start failed");
            redirect_with_error(&state, "internal")
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    /// Google sends `?error=access_denied` when the user clicks "Cancel"
    /// on the consent screen.
    pub error: Option<String>,
}

/// `GET /auth/google/callback?code=...&state=...` — exchanges the code,
/// runs the GoogleAuth use case, sets the refresh cookie and 302s the
/// browser to the SPA. On any failure we redirect to the error route
/// with `?oauth_error=<code>` so the SPA can surface a banner.
pub async fn callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(q): Query<CallbackQuery>,
) -> Response {
    let Some(google) = state.google_auth.as_ref() else {
        return feature_disabled();
    };

    // The browser may have been redirected back with `?error=...` (user
    // declined consent or revoked at Google). Surface it as `denied`.
    if let Some(err) = q.error {
        warn!(provider_error = %err, "google callback returned error param");
        return redirect_with_error(&state, "denied");
    }

    let (Some(code), Some(csrf_state)) = (q.code, q.state) else {
        return redirect_with_error(&state, "bad_request");
    };

    let out = match google.callback(&code, &csrf_state).await {
        Ok(out) => out,
        Err(e) => {
            let kind = classify(&e);
            warn!(error = %e, kind = %kind, "google /callback failed");
            return redirect_with_error(&state, kind);
        }
    };

    let max_age = (out.tokens.refresh_expires_at - Utc::now())
        .num_seconds()
        .max(0);
    let jar = jar
        // Defensive: drop any stale cookie from a different account before
        // attaching the fresh one.
        .add(clear_refresh_cookie())
        .add(refresh_cookie(out.tokens.refresh_token, max_age));

    let url = state
        .google_post_login_redirect
        .as_deref()
        .unwrap_or("/dashboard");

    (jar, Redirect::to(url)).into_response()
}

/// Stable error codes that the SPA can recognise on the `/login?oauth_error=...`
/// landing. Kept small and deliberately vague — we don't want to leak
/// "this email exists" or "this google account is already linked" via the
/// redirect.
fn classify(e: &GoogleAuthError) -> &'static str {
    match e {
        GoogleAuthError::OAuth(OAuthError::InvalidState) => "expired",
        GoogleAuthError::OAuth(OAuthError::EmailNotVerified) => "unverified",
        GoogleAuthError::LinkRefused => "unverified",
        GoogleAuthError::OAuth(OAuthError::Network(_)) => "network",
        GoogleAuthError::OAuth(OAuthError::Provider(_))
        | GoogleAuthError::Domain(_)
        | GoogleAuthError::Repo(_)
        | GoogleAuthError::Cache(_)
        | GoogleAuthError::Session(_) => "internal",
    }
}

fn redirect_with_error(state: &AppState, code: &str) -> Response {
    let base = state
        .google_error_redirect
        .as_deref()
        .unwrap_or("/login");
    let sep = if base.contains('?') { '&' } else { '?' };
    let target = format!("{base}{sep}oauth_error={code}");
    Redirect::to(&target).into_response()
}

fn feature_disabled() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "google oauth is not configured",
    )
        .into_response()
}
