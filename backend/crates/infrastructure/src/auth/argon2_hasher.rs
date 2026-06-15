use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash as ArgonPhc, SaltString},
    Argon2, PasswordHasher as ArgonHasher, PasswordVerifier,
};
use async_trait::async_trait;
use tokio::sync::OnceCell;

use application::ports::{HasherError, PasswordHasher};
use domain::PasswordHash;

/// A pre-computed PHC string with the same params as our real hashes. Used by
/// `dummy_verify` to spend the same time on missing-user login attempts as on
/// real ones. Initialised on first use; the lock keeps the hash work to once.
static DUMMY_HASH: OnceCell<PasswordHash> = OnceCell::const_new();

const DUMMY_PASSWORD: &str = "argon2-timing-mitigation-dummy-password";

/// Argon2id hasher. Each call runs on `spawn_blocking` because argon2 is
/// CPU-intensive by design — running it on the async executor would stall
/// other requests.
#[derive(Default, Clone)]
pub struct Argon2Hasher;

impl Argon2Hasher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PasswordHasher for Argon2Hasher {
    async fn hash(&self, password: &str) -> Result<PasswordHash, HasherError> {
        let password = password.to_owned();
        tokio::task::spawn_blocking(move || -> Result<PasswordHash, HasherError> {
            let salt = SaltString::generate(&mut OsRng);
            let argon = Argon2::default();
            let phc = argon
                .hash_password(password.as_bytes(), &salt)
                .map_err(|e| HasherError::Hashing(e.to_string()))?
                .to_string();
            Ok(PasswordHash::from_raw(phc))
        })
        .await
        .map_err(|e| HasherError::Hashing(format!("join error: {e}")))?
    }

    async fn verify(&self, password: &str, hash: &PasswordHash) -> Result<bool, HasherError> {
        let password = password.to_owned();
        let hash_str = hash.as_str().to_owned();
        tokio::task::spawn_blocking(move || -> Result<bool, HasherError> {
            let parsed =
                ArgonPhc::new(&hash_str).map_err(|e| HasherError::Verification(e.to_string()))?;
            Ok(Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok())
        })
        .await
        .map_err(|e| HasherError::Verification(format!("join error: {e}")))?
    }

    async fn dummy_verify(&self, password: &str) -> Result<(), HasherError> {
        let dummy = DUMMY_HASH
            .get_or_init(|| async { self.hash(DUMMY_PASSWORD).await.expect("dummy hash failed") })
            .await
            .clone();
        // Verification will return false; we only care about the time it takes.
        let _ = self.verify(password, &dummy).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hash_then_verify_succeeds() {
        let h = Argon2Hasher::new();
        let hash = h.hash("correcthorsebatterystaple").await.unwrap();
        assert!(h.verify("correcthorsebatterystaple", &hash).await.unwrap());
        assert!(!h.verify("wrong-password", &hash).await.unwrap());
    }
}
