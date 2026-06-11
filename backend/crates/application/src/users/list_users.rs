use std::sync::Arc;

use domain::User;

use crate::ports::{RepoError, UserRepository};

pub struct ListUsers {
    repo: Arc<dyn UserRepository>,
}

impl ListUsers {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    pub async fn execute(&self, limit: i64, offset: i64) -> Result<Vec<User>, RepoError> {
        let limit = limit.clamp(1, 100);
        let offset = offset.max(0);
        self.repo.list(limit, offset).await
    }
}
