//! Application layer — use cases and ports (traits) the domain depends on.
//! Knows nothing about HTTP, SQL, or Redis; only abstract interfaces.

pub mod auth;
pub mod ports;
pub mod users;

pub use ports::{
    AuthRequest, CacheError, CacheStore, GoogleAuthClient, GoogleUserInfo, HasherError, OAuthError,
    PasswordHasher, RepoError, SessionError, SessionManager, TokenPair, UserRepository,
};
