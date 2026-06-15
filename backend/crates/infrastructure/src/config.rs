use std::net::SocketAddr;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub bind_addr: SocketAddr,
    pub log_level: String,
    pub db_max_connections: u32,
    pub auth: AuthConfig,
    pub google: Option<GoogleConfig>,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

/// Google OAuth — populated only if both `GOOGLE_CLIENT_ID` and
/// `GOOGLE_CLIENT_SECRET` are set. With neither, the `/auth/google/*`
/// routes return 503 (the feature is just off).
#[derive(Debug, Clone)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
    /// Must exactly match what's registered in the Google OAuth console.
    /// Dev default: `http://localhost:5173/api/auth/google/callback`
    /// (routed through edge nginx → backend).
    pub redirect_uri: String,
    /// Where the user lands after a successful callback. Browser sees
    /// the edge origin, so a path-relative string is fine.
    pub post_login_redirect: String,
    /// Where the user lands when something goes wrong. Path-relative.
    pub error_redirect: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let _ = dotenvy::dotenv();

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL is required"))?;
        let redis_url =
            std::env::var("REDIS_URL").map_err(|_| anyhow::anyhow!("REDIS_URL is required"))?;

        let bind_addr: SocketAddr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:8080".into())
            .parse()
            .map_err(|e| anyhow::anyhow!("BIND_ADDR is not a valid socket address: {e}"))?;

        let log_level =
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,api=debug,tower_http=info".into());

        let db_max_connections = parse_env("DB_MAX_CONNECTIONS", 10)?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .map_err(|_| anyhow::anyhow!("JWT_SECRET is required (>= 32 bytes)"))?;
        if jwt_secret.len() < 32 {
            anyhow::bail!("JWT_SECRET must be at least 32 bytes");
        }
        let jwt_issuer = std::env::var("JWT_ISSUER").unwrap_or_else(|_| "rust-react-api".into());

        let access_ttl_secs: u64 = parse_env("ACCESS_TTL_SECS", 900)?; // 15 min
        let refresh_ttl_secs: u64 = parse_env("REFRESH_TTL_SECS", 60 * 60 * 24 * 30)?; // 30 days

        let google = match (
            std::env::var("GOOGLE_CLIENT_ID")
                .ok()
                .filter(|s| !s.is_empty()),
            std::env::var("GOOGLE_CLIENT_SECRET")
                .ok()
                .filter(|s| !s.is_empty()),
        ) {
            (Some(client_id), Some(client_secret)) => Some(GoogleConfig {
                client_id,
                client_secret,
                redirect_uri: std::env::var("GOOGLE_REDIRECT_URI")
                    .unwrap_or_else(|_| "http://localhost:5173/api/auth/google/callback".into()),
                post_login_redirect: std::env::var("OAUTH_POST_LOGIN_REDIRECT")
                    .unwrap_or_else(|_| "/dashboard".into()),
                error_redirect: std::env::var("OAUTH_ERROR_REDIRECT")
                    .unwrap_or_else(|_| "/login".into()),
            }),
            _ => None,
        };

        Ok(Self {
            database_url,
            redis_url,
            bind_addr,
            log_level,
            db_max_connections,
            auth: AuthConfig {
                jwt_secret,
                jwt_issuer,
                access_ttl: Duration::from_secs(access_ttl_secs),
                refresh_ttl: Duration::from_secs(refresh_ttl_secs),
            },
            google,
        })
    }
}

fn parse_env<T>(name: &str, default: T) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(name) {
        Ok(v) => v
            .parse::<T>()
            .map_err(|e| anyhow::anyhow!("{name} is not parseable: {e}")),
        Err(_) => Ok(default),
    }
}
