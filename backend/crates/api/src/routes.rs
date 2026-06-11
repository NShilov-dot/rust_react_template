use std::sync::Arc;
use std::time::Duration;

use axum::{
    http::{header, HeaderValue},
    routing::{get, post},
    Router,
};
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

use crate::handlers;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    // ─── Per-IP rate limits on auth endpoints ──────────────────────────
    // Token-bucket: `burst` initial tokens, +1 per `period`. SmartIpKeyExtractor
    // honours Forwarded / X-Forwarded-For (nginx sets it), falling back to
    // ConnectInfo for direct connections.
    //
    // Inlined rather than factored into a helper because the full middleware
    // type (`NoOpMiddleware<QuantaInstant>`) is awkward to spell in a return
    // signature; type inference from `.finish()` handles it for us when we
    // pipe the value straight into `GovernorLayer`.
    let login_rl = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(12)) // ≈ 5/min sustained
            .burst_size(5)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("valid governor config"),
    );
    let register_rl = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(20)) // ≈ 3/min sustained
            .burst_size(3)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("valid governor config"),
    );
    let refresh_rl = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(2)) // ≈ 30/min sustained
            .burst_size(30)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("valid governor config"),
    );
    // Google flow: looser than login (browser-driven, the user may click
    // a couple of times) but still IP-bounded so a denial-of-service
    // attempt against Google can't ride our quota.
    let oauth_rl = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(6)) // ≈ 10/min sustained
            .burst_size(10)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("valid governor config"),
    );

    let login = Router::new()
        .route("/auth/login", post(handlers::auth::login))
        .layer(GovernorLayer::new(login_rl));

    let register = Router::new()
        .route("/auth/register", post(handlers::auth::register))
        .layer(GovernorLayer::new(register_rl));

    let refresh = Router::new()
        .route("/auth/refresh", post(handlers::auth::refresh))
        .layer(GovernorLayer::new(refresh_rl));

    let oauth = Router::new()
        .route("/auth/google/start", get(handlers::google::start))
        .route("/auth/google/callback", get(handlers::google::callback))
        .layer(GovernorLayer::new(oauth_rl));

    // ─── Routes without rate limit ─────────────────────────────────────
    let unlimited = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/me", get(handlers::auth::me))
        .route("/users", get(handlers::users::list))
        .route("/users/{id}", get(handlers::users::get));

    Router::new()
        .merge(login)
        .merge(register)
        .merge(refresh)
        .merge(oauth)
        .merge(unlimited)
        // ─── Global response headers ───────────────────────────────────
        // `nosniff` prevents browsers from MIME-sniffing JSON as HTML/JS.
        // The HTML-specific headers (CSP, X-Frame-Options) live on nginx
        // where the SPA shell is served.
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
