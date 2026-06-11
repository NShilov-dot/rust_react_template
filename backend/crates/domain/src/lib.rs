//! Domain layer — pure business types. No I/O, no framework deps.

pub mod errors;
pub mod password;
pub mod user;

pub use errors::DomainError;
pub use password::{Password, PasswordHash};
pub use user::{Email, User, UserId};
