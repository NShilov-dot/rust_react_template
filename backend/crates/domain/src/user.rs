use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let raw = raw.into();
        let trimmed = raw.trim();
        let at_count = trimmed.matches('@').count();
        if at_count != 1 || trimmed.len() > 254 || trimmed.starts_with('@') || trimmed.ends_with('@') {
            return Err(DomainError::InvalidEmail(raw));
        }
        Ok(Self(trimmed.to_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: Email,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(email: Email, name: String) -> Result<Self, DomainError> {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.len() > 100 {
            return Err(DomainError::InvalidName(name));
        }
        let now = Utc::now();
        Ok(Self {
            id: UserId::new(),
            email,
            name: trimmed.to_string(),
            created_at: now,
            updated_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_normalizes_to_lowercase() {
        let email = Email::parse("Foo@Bar.COM").unwrap();
        assert_eq!(email.as_str(), "foo@bar.com");
    }

    #[test]
    fn email_rejects_missing_at_sign() {
        assert!(Email::parse("nope").is_err());
    }

    #[test]
    fn user_rejects_blank_name() {
        let email = Email::parse("a@b.co").unwrap();
        assert!(User::new(email, "   ".into()).is_err());
    }
}
