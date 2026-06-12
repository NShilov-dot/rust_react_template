use std::sync::Arc;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use application::auth::{
    google::GoogleAuth, login::Login, logout::Logout, refresh::Refresh, register::Register,
};
use application::ports::{
    GoogleAuthClient, PasswordHasher, SessionManager, TaskRepository, UserRepository,
};
use application::tasks::{
    create_task::CreateTask, delete_task::DeleteTask, get_task::GetTask, list_tasks::ListTasks,
    update_task::UpdateTask,
};
use application::users::{get_user::GetUser, list_users::ListUsers};
use infrastructure::{
    auth::{Argon2Hasher, RedisJwtSessions, SessionConfig},
    cache::RedisCache,
    config::Config,
    oauth::GoogleOAuthClient,
    postgres::{self, task_repository::PgTaskRepository, user_repository::PgUserRepository},
};

mod error;
mod extractors;
mod handlers;
mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;

    tracing_subscriber::registry()
        .with(EnvFilter::try_new(&config.log_level).unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("connecting to postgres");
    let pool = postgres::connect(&config.database_url, config.db_max_connections).await?;

    tracing::info!("running migrations");
    sqlx::migrate!("../../migrations").run(&pool).await?;

    tracing::info!("connecting to redis");
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis_conn = redis::aio::ConnectionManager::new(redis_client).await?;

    let cache: Arc<dyn application::ports::CacheStore> =
        Arc::new(RedisCache::from_connection(redis_conn.clone()));

    let sessions: Arc<dyn SessionManager> = Arc::new(RedisJwtSessions::new(
        redis_conn,
        SessionConfig {
            jwt_secret: config.auth.jwt_secret.clone(),
            jwt_issuer: config.auth.jwt_issuer.clone(),
            access_ttl: config.auth.access_ttl,
            refresh_ttl: config.auth.refresh_ttl,
        },
    )?);

    let user_repo: Arc<dyn UserRepository> = Arc::new(PgUserRepository::new(pool.clone()));
    let task_repo: Arc<dyn TaskRepository> = Arc::new(PgTaskRepository::new(pool));
    let hasher: Arc<dyn PasswordHasher> = Arc::new(Argon2Hasher::new());

    let (google_auth, google_post_login_redirect, google_error_redirect) = match config.google {
        Some(g) => {
            tracing::info!("google oauth enabled");
            let client: Arc<dyn GoogleAuthClient> = Arc::new(GoogleOAuthClient::new(
                g.client_id,
                g.client_secret,
                g.redirect_uri,
            )?);
            let use_case = Arc::new(GoogleAuth::new(
                user_repo.clone(),
                cache.clone(),
                client,
                sessions.clone(),
            ));
            (
                Some(use_case),
                Some(g.post_login_redirect),
                Some(g.error_redirect),
            )
        }
        None => {
            tracing::info!("google oauth disabled (GOOGLE_CLIENT_ID/SECRET not set)");
            (None, None, None)
        }
    };

    let state = AppState {
        register: Arc::new(Register::new(user_repo.clone(), hasher.clone(), sessions.clone())),
        login: Arc::new(Login::new(user_repo.clone(), hasher.clone(), sessions.clone())),
        refresh: Arc::new(Refresh::new(sessions.clone())),
        logout: Arc::new(Logout::new(sessions.clone())),
        get_user: Arc::new(GetUser::new(user_repo.clone(), cache.clone())),
        list_users: Arc::new(ListUsers::new(user_repo.clone())),
        create_task: Arc::new(CreateTask::new(task_repo.clone())),
        list_tasks: Arc::new(ListTasks::new(task_repo.clone())),
        get_task: Arc::new(GetTask::new(task_repo.clone())),
        update_task: Arc::new(UpdateTask::new(task_repo.clone())),
        delete_task: Arc::new(DeleteTask::new(task_repo.clone())),
        sessions,
        google_auth,
        google_post_login_redirect,
        google_error_redirect,
    };

    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "listening");
    // `into_make_service_with_connect_info::<SocketAddr>` exposes the peer
    // address to extractors — required for `SmartIpKeyExtractor` to fall back
    // to ConnectInfo when no `X-Forwarded-For` is present (e.g. direct curl).
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
