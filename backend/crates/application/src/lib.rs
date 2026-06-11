//! Application layer — use cases and ports (traits) the domain depends on.
//! Knows nothing about HTTP, SQL, or Redis; only abstract interfaces.

pub mod auth;
pub mod ports;
pub mod users;

pub use ports::{
    CacheError, CacheStore, HasherError, PasswordHasher, RepoError, SessionError, SessionManager,
    TokenPair, UserRepository,
};
