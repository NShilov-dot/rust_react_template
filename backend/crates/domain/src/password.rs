use serde::{Deserialize, Serialize};

use crate::errors::DomainError;

/// Raw password from user input. Validated for minimum strength.
/// Wrapped so it can't accidentally be logged or serialized.
#[derive(Clone)]
pub struct Password(String);

impl std::fmt::Debug for Password {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Password(***)")
    }
}

impl Password {
    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let raw = raw.into();
        if raw.len() < 8 {
            return Err(DomainError::WeakPassword(
                "password must be at least 8 characters".into(),
            ));
        }
        if raw.len() > 256 {
            return Err(DomainError::WeakPassword(
                "password must be at most 256 characters".into(),
            ));
        }
        Ok(Self(raw))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// PHC-format hash string produced by the password hasher.
/// Opaque to the domain — only the hasher port knows how to verify it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordHash(String);

impl std::fmt::Debug for PasswordHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PasswordHash(***)")
    }
}

impl PasswordHash {
    /// Wrap a previously-computed hash (e.g. read from the database).
    pub fn from_raw(raw: impl Into<String>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
