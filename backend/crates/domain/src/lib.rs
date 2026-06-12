//! Domain layer — pure business types. No I/O, no framework deps.

pub mod errors;
pub mod password;
pub mod task;
pub mod user;

pub use errors::DomainError;
pub use password::{Password, PasswordHash};
pub use task::{Task, TaskDescription, TaskId, TaskPriority, TaskStatus, TaskTitle};
pub use user::{Email, User, UserId};
