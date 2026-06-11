use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

use domain::{Email, PasswordHash, User, UserId};

// ─── Repository ──────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("storage error: {0}")]
    Storage(String),
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: &User, password_hash: &PasswordHash) -> Result<(), RepoError>;
    async fn get(&self, id: UserId) -> Result<User, RepoError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepoError>;
    /// Returns the user and the stored password hash for auth flows.
    async fn find_for_login(
        &self,
        email: &Email,
    ) -> Result<Option<(User, PasswordHash)>, RepoError>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<User>, RepoError>;
}

// ─── Cache ───────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("cache backend error: {0}")]
    Backend(String),
}

#[async_trait]
pub trait CacheStore: Send + Sync {
    async fn get_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;
    async fn set_bytes(
        &self,
        key: &str,
        value: &[u8],
        ttl_secs: Option<u64>,
    ) -> Result<(), CacheError>;
    async fn delete(&self, key: &str) -> Result<(), CacheError>;
}

// ─── Password hashing ────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum HasherError {
    #[error("hashing failed: {0}")]
    Hashing(String),
    #[error("verification failed: {0}")]
    Verification(String),
}

#[async_trait]
pub trait PasswordHasher: Send + Sync {
    async fn hash(&self, password: &str) -> Result<PasswordHash, HasherError>;
    async fn verify(&self, password: &str, hash: &PasswordHash) -> Result<bool, HasherError>;
    /// Run a verify against a precomputed dummy hash. Used by `Login` when
    /// the email is unknown — equalises response time so a remote attacker
    /// can't enumerate registered users by timing.
    async fn dummy_verify(&self, password: &str) -> Result<(), HasherError>;
}

// ─── Sessions / tokens ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("invalid or expired token")]
    Invalid,
    #[error("session backend error: {0}")]
    Backend(String),
}

/// Issues, rotates, verifies and revokes access/refresh tokens.
///
/// The implementation is responsible for:
/// - signing/verifying short-lived access tokens (stateless JWT),
/// - issuing opaque refresh tokens scoped to a session "family",
/// - atomic rotation with reuse detection: presenting a previously-rotated
///   refresh token revokes the entire family.
#[async_trait]
pub trait SessionManager: Send + Sync {
    async fn issue(&self, user_id: UserId) -> Result<TokenPair, SessionError>;
    async fn rotate(&self, refresh_token: &str) -> Result<TokenPair, SessionError>;
    fn verify_access(&self, access_token: &str) -> Result<UserId, SessionError>;
    async fn revoke(&self, refresh_token: &str) -> Result<(), SessionError>;
}
