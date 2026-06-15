use std::sync::Arc;

use thiserror::Error;

use domain::{DomainError, Email, Password, User};

use crate::ports::{
    HasherError, PasswordHasher, RepoError, SessionError, SessionManager, TokenPair, UserRepository,
};

#[derive(Debug, Error)]
pub enum RegisterError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Hasher(#[from] HasherError),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error("email already taken")]
    EmailTaken,
}

#[derive(Debug, Clone)]
pub struct RegisterInput {
    pub email: String,
    pub name: String,
    pub password: String,
}

pub struct RegisterOutput {
    pub user: User,
    pub tokens: TokenPair,
}

pub struct Register {
    repo: Arc<dyn UserRepository>,
    hasher: Arc<dyn PasswordHasher>,
    sessions: Arc<dyn SessionManager>,
}

impl Register {
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

    pub async fn execute(&self, input: RegisterInput) -> Result<RegisterOutput, RegisterError> {
        let email = Email::parse(input.email)?;
        let password = Password::parse(input.password)?;

        if self.repo.find_by_email(&email).await?.is_some() {
            return Err(RegisterError::EmailTaken);
        }

        let user = User::new(email, input.name)?;
        let hash = self.hasher.hash(password.expose()).await?;
        self.repo.create(&user, &hash).await?;

        let tokens = self.sessions.issue(user.id).await?;
        Ok(RegisterOutput { user, tokens })
    }
}
