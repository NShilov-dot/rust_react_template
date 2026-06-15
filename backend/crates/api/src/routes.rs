use std::sync::Arc;
use std::time::Duration;

use axum::{
    http::{header, HeaderValue},
    routing::{get, post},
    Router,
};
use axum_prometheus::PrometheusMetricLayerBuilder;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

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
    // Tasks write path: caps how fast an authenticated client can spam
    // INSERTs/UPDATEs/DELETEs. Per-IP rather than per-user (per-user would
    // need a custom key extractor reading the JWT — overkill for an MVP).
    // Reads are not gated: list/get are cheap and a logged-in user
    // legitimately calls them on every navigation.
    let tasks_write_rl = Arc::new(
        GovernorConfigBuilder::default()
            .period(Duration::from_secs(1)) // ≈ 60/min sustained
            .burst_size(30)
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

    // ─── Prometheus RED metrics ───────────────────────────────────────
    // The layer auto-instruments every request: emits
    //   axum_http_requests_total{method,endpoint,status}
    //   axum_http_requests_duration_seconds_bucket{le,...}
    //   axum_http_requests_pending{method,endpoint}
    // The handle is what /metrics queries to render the Prom text.
    // `with_ignore_pattern` keeps the scrape itself out of its own metrics.
    let (prom_layer, prom_handle) = PrometheusMetricLayerBuilder::new()
        .with_ignore_patterns(&["/metrics", "/health"])
        .with_default_metrics()
        .build_pair();

    let oauth = Router::new()
        .route("/auth/google/start", get(handlers::google::start))
        .route("/auth/google/callback", get(handlers::google::callback))
        .layer(GovernorLayer::new(oauth_rl));

    // Writes go through the rate-limited router; reads stay in `unlimited`
    // alongside the other GETs. axum merges these two routers on the same
    // path because they declare different HTTP methods.
    let tasks_write = Router::new()
        .route("/tasks", post(handlers::tasks::create))
        .route(
            "/tasks/{id}",
            axum::routing::patch(handlers::tasks::update)
                .delete(handlers::tasks::delete),
        )
        .layer(GovernorLayer::new(tasks_write_rl));

    // ─── Routes without rate limit ─────────────────────────────────────
    let unlimited = Router::new()
        .route("/health", get(handlers::health::health))
        .route(
            "/metrics",
            get(move || std::future::ready(prom_handle.render())),
        )
        .route("/auth/logout", post(handlers::auth::logout))
        .route("/auth/me", get(handlers::auth::me))
        .route("/users", get(handlers::users::list))
        .route("/users/{id}", get(handlers::users::get))
        .route("/tasks", get(handlers::tasks::list))
        .route("/tasks/{id}", get(handlers::tasks::get));

    Router::new()
        .merge(login)
        .merge(register)
        .merge(refresh)
        .merge(oauth)
        .merge(tasks_write)
        .merge(unlimited)
        // ─── Global response headers ───────────────────────────────────
        // `nosniff` prevents browsers from MIME-sniffing JSON as HTML/JS.
        // The HTML-specific headers (CSP, X-Frame-Options) live on nginx
        // where the SPA shell is served.
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        // Order matters: TraceLayer outermost so spans cover the whole
        // request including the prometheus layer's overhead.
        .layer(prom_layer)
        // INFO-level spans (default would be DEBUG) so they survive a
        // production-grade `RUST_LOG=info` filter and reach the OTLP
        // exporter for trace export.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state)
}
