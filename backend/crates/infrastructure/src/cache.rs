use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands, Client};

use application::ports::{CacheError, CacheStore};

#[derive(Clone)]
pub struct RedisCache {
    conn: ConnectionManager,
}

impl RedisCache {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let client = Client::open(url)?;
        let conn = ConnectionManager::new(client).await?;
        Ok(Self { conn })
    }

    pub fn from_connection(conn: ConnectionManager) -> Self {
        Self { conn }
    }
}

fn map_err(e: redis::RedisError) -> CacheError {
    CacheError::Backend(e.to_string())
}

#[async_trait]
impl CacheStore for RedisCache {
    async fn get_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        let mut conn = self.conn.clone();
        let value: Option<Vec<u8>> = conn.get(key).await.map_err(map_err)?;
        Ok(value)
    }

    async fn set_bytes(
        &self,
        key: &str,
        value: &[u8],
        ttl_secs: Option<u64>,
    ) -> Result<(), CacheError> {
        let mut conn = self.conn.clone();
        match ttl_secs {
            Some(ttl) => {
                let _: () = conn.set_ex(key, value, ttl).await.map_err(map_err)?;
            }
            None => {
                let _: () = conn.set(key, value).await.map_err(map_err)?;
            }
        }
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), CacheError> {
        let mut conn = self.conn.clone();
        let _: () = conn.del(key).await.map_err(map_err)?;
        Ok(())
    }
}
