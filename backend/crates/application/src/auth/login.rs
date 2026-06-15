use std::sync::Arc;

use thiserror::Error;

use domain::{Email, User};

use crate::ports::{
    HasherError, PasswordHasher, RepoError, SessionError, SessionManager, TokenPair, UserRepository,
};

#[derive(Debug, Error)]
pub enum LoginError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Hasher(#[from] HasherError),
    #[error(transparent)]
    Session(#[from] SessionError),
}

#[derive(Debug, Clone)]
pub struct LoginInput {
    pub email: String,
    pub password: String,
}

pub struct LoginOutput {
    pub user: User,
    pub tokens: TokenPair,
}

pub struct Login {
    repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    sessions: Arc<dyn SessionManager>,
}

impl Login {
    pub fn new(
        repo: Arc<dyn UserRepository>,
        hasher: Arc<dyn PasswordHasher>,
        sessions: Arc<dyn SessionManager>,
    ) -> Self {
        Self {
            repo,
            hasher,
            sessions,
        }
    }

    pub async fn execute(&self, input: LoginInput) -> Result<LoginOutput, LoginError> {
        let email = Email::parse(input.email).map_err(|_| LoginError::InvalidCredentials)?;

        let Some((user, hash)) = self.repo.find_for_login(&email).await? else {
            // Timing-attack mitigation: spend ~the same time as a real verify
            // would, so attackers can't enumerate registered emails by timing
            // /auth/login responses.
            let _ = self.hasher.dummy_verify(&input.password).await;
            return Err(LoginError::InvalidCredentials);
        };

        if !self.hasher.verify(&input.password, &hash).await? {
            return Err(LoginError::InvalidCredentials);
        }

        let tokens = self.sessions.issue(user.id).await?;
        Ok(LoginOutput { user, tokens })
    }
}
