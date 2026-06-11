use std::sync::Arc;

use domain::{User, UserId};

use crate::ports::{CacheStore, RepoError, UserRepository};

const CACHE_TTL_SECS: u64 = 300;

pub struct GetUser {
    repo: Arc<dyn UserRepository>,
    cache: Arc<dyn CacheStore>,
}

impl GetUser {
    pub fn new(repo: Arc<dyn UserRepository>, cache: Arc<dyn CacheStore>) -> Self {
        Self { repo, cache }
    }

    pub async fn execute(&self, id: UserId) -> Result<User, RepoError> {
        let key = cache_key(id);

        match self.cache.get_bytes(&key).await {
            Ok(Some(bytes)) => match serde_json::from_slice::<User>(&bytes) {
                Ok(user) => return Ok(user),
                Err(e) => tracing::warn!(error = %e, "cached user payload was invalid, evicting"),
            },
            Ok(None) => {}
            Err(e) => tracing::warn!(error = %e, "cache read failed; falling through to repo"),
        }

        let user = self.repo.get(id).await?;

        if let Ok(bytes) = serde_json::to_vec(&user) {
            if let Err(e) = self.cache.set_bytes(&key, &bytes, Some(CACHE_TTL_SECS)).await {
                tracing::warn!(error = %e, "failed to populate cache");
            }
        }

        Ok(user)
    }
}

fn cache_key(id: UserId) -> String {
    format!("user:{}", id.0)
}
