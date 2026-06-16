use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

use uuid::Uuid;

use domain::{Email, PasswordHash, Task, TaskId, TaskStatus, User, UserId};

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
    /// Create a brand-new user from OAuth (no password). `google_id` is the
    /// stable Google subject claim (`sub`), NOT the email.
    async fn create_oauth(&self, user: &User, google_id: &str) -> Result<(), RepoError>;
    async fn get(&self, id: UserId) -> Result<User, RepoError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepoError>;
    async fn find_by_google_id(&self, google_id: &str) -> Result<Option<User>, RepoError>;
    /// Attach a `google_id` to an already-existing user (e.g. password user
    /// signs in with Google for the first time and we auto-link by verified
    /// email). Fails with Conflict if the user already has a different one.
    async fn link_google(&self, user_id: UserId, google_id: &str) -> Result<(), RepoError>;
    /// Returns the user and the stored password hash for password auth.
    /// Returns None for OAuth-only users (NULL password_hash in DB).
    async fn find_for_login(
        &self,
        email: &Email,
    ) -> Result<Option<(User, PasswordHash)>, RepoError>;
    async fn list(&self, limit: i64, offset: i64) -> Result<Vec<User>, RepoError>;
}

// ─── Task repository ─────────────────────────────────────────────────────

/// Filters for `TaskRepository::list_for_owner`. All filters compose with
/// the implicit `owner_id = $1` predicate enforced at the SQL level.
///
/// `cursor` is the `(created_at, id)` of the last item from the previous
/// page — `None` means "give me the first page". The repository implements
/// keyset pagination (`WHERE (created_at, id) < cursor`), so latency is
/// constant regardless of how deep into the list we scroll. Offset-based
/// pagination would slow down quadratically.
#[derive(Debug, Default, Clone)]
pub struct TaskListFilter {
    pub status: Option<TaskStatus>,
    pub cursor: Option<(DateTime<Utc>, Uuid)>,
    pub limit: i64,
}

#[async_trait]
pub trait TaskRepository: Send + Sync {
    async fn create(&self, task: &Task) -> Result<(), RepoError>;
    /// Returns the task only if it belongs to `owner_id` — keeps IDOR out
    /// of the handler's responsibility.
    async fn find_for_owner(&self, id: TaskId, owner_id: UserId)
        -> Result<Option<Task>, RepoError>;
    async fn list_for_owner(
        &self,
        owner_id: UserId,
        filter: TaskListFilter,
    ) -> Result<Vec<Task>, RepoError>;
    /// Persists every mutable field. The caller is responsible for having
    /// loaded the task via `find_for_owner` first, so the `owner_id`
    /// check has already been done.
    async fn update(&self, task: &Task) -> Result<(), RepoError>;
    /// Deletes only if the task belongs to `owner_id`. Returns `NotFound`
    /// when the row is absent OR owned by someone else (don't leak which).
    async fn delete(&self, id: TaskId, owner_id: UserId) -> Result<(), RepoError>;
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

// ─── OAuth — Google ──────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("invalid or expired state")]
    InvalidState,
    #[error("provider returned no/invalid response: {0}")]
    Provider(String),
    #[error("email not verified by provider")]
    EmailNotVerified,
    #[error("network error: {0}")]
    Network(String),
}

/// Output of `authorization_url` — the URL to redirect the browser to plus
/// the secrets we need to persist for the callback. The implementation
/// generates a CSRF state token, PKCE verifier, and OIDC nonce; the
/// application layer must stash both `pkce_verifier` and `nonce` under
/// the state key in a cache store with a short TTL so we can rebind them
/// to the id_token returned on callback.
#[derive(Debug)]
pub struct AuthRequest {
    pub authorize_url: String,
    pub csrf_state: String,
    pub pkce_verifier: String,
    /// OIDC `nonce` — random per-authorize secret. Included in the
    /// authorize URL and must equal the `nonce` claim in the id_token
    /// returned at the token endpoint. Protects against id_token replay.
    pub nonce: String,
}

/// User profile data we trust the provider to give us. `sub` is Google's
/// stable user ID — never the email. Use `email_verified` to decide
/// auto-link safety.
#[derive(Debug, Clone)]
pub struct GoogleUserInfo {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
}

#[async_trait]
pub trait GoogleAuthClient: Send + Sync {
    /// Build the Google authorize URL with PKCE + state + nonce. Returns
    /// secrets the caller must store before the redirect.
    fn authorize(&self) -> AuthRequest;

    /// Exchange `code` for tokens and return the verified subject claims.
    /// The implementation MUST verify the id_token's signature against
    /// Google's JWKS and check `iss`, `aud`, `exp`, and that the `nonce`
    /// claim equals `expected_nonce`. Userinfo endpoint MUST NOT be the
    /// source of identity claims — only id_token.
    async fn exchange(
        &self,
        code: &str,
        pkce_verifier: &str,
        expected_nonce: &str,
    ) -> Result<GoogleUserInfo, OAuthError>;
}
