//! Session manager: JWT (HS256) access tokens + opaque, rotating refresh tokens.
//!
//! ## Refresh token format
//!
//! `{family_id_hex}.{base64url_random}` — 16 bytes hex + dot + 32 random bytes.
//! Encoding the family_id in the token itself lets us locate the correct
//! Redis keys without an extra lookup.
//!
//! ## Redis keys
//!
//! - `refresh:{token}` → `user_id` (string). TTL = refresh_ttl.
//! - `family:{family_id}:current` → currently-valid token in this family.
//! - `family:{family_id}:revoked` → `"1"` if reuse was detected.
//!
//! ## Rotation (atomic via Lua)
//!
//! On rotate: if the presented token isn't the current one for its family,
//! the family is marked revoked and the rotation rejected — this catches
//! refresh-token theft, since the legitimate client and the attacker can't
//! both hold the "current" token at once.

use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, TimeDelta, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use application::ports::{SessionError, SessionManager, TokenPair};
use domain::UserId;

const ROTATE_LUA: &str = r#"
if redis.call('EXISTS', KEYS[3]) == 1 then
    return redis.error_reply('family_revoked')
end
local user_id = redis.call('GET', KEYS[1])
if not user_id then
    return redis.error_reply('not_found')
end
local current = redis.call('GET', KEYS[2])
if current ~= ARGV[1] then
    redis.call('DEL', KEYS[1])
    redis.call('DEL', KEYS[2])
    redis.call('SET', KEYS[3], '1', 'EX', ARGV[3])
    return redis.error_reply('reuse_detected')
end
redis.call('DEL', KEYS[1])
redis.call('SET', KEYS[4], user_id, 'EX', ARGV[3])
redis.call('SET', KEYS[2], ARGV[2], 'EX', ARGV[3])
return user_id
"#;

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessClaims {
    sub: String,
    iss: String,
    iat: i64,
    exp: i64,
}

pub struct RedisJwtSessions {
    redis: ConnectionManager,
    encoding: EncodingKey,
    decoding: DecodingKey,
    config: SessionConfig,
    rotate_script: redis::Script,
}

impl RedisJwtSessions {
    pub fn new(redis: ConnectionManager, config: SessionConfig) -> anyhow::Result<Self> {
        if config.jwt_secret.len() < 32 {
            anyhow::bail!("JWT_SECRET must be at least 32 bytes");
        }
        let encoding = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let decoding = DecodingKey::from_secret(config.jwt_secret.as_bytes());
        let rotate_script = redis::Script::new(ROTATE_LUA);
        Ok(Self {
            redis,
            encoding,
            decoding,
            config,
            rotate_script,
        })
    }

    fn random_refresh(family_id: Uuid) -> String {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let rand_part = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
        format!("{}.{}", family_id.as_simple(), rand_part)
    }

    fn parse_family(token: &str) -> Result<Uuid, SessionError> {
        let (head, _) = token.split_once('.').ok_or(SessionError::Invalid)?;
        Uuid::parse_str(head).map_err(|_| SessionError::Invalid)
    }

    fn sign_access(
        &self,
        user_id: UserId,
        now: DateTime<Utc>,
        exp: DateTime<Utc>,
    ) -> Result<String, SessionError> {
        let claims = AccessClaims {
            sub: user_id.0.to_string(),
            iss: self.config.jwt_issuer.clone(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| SessionError::Backend(e.to_string()))
    }

    fn map_redis(e: redis::RedisError) -> SessionError {
        // Connection / IO errors are transient backend failures. Anything
        // else (response errors, Lua script `error_reply`) means the token
        // is logically invalid — don't leak which case it was to the caller.
        if e.is_connection_dropped() || e.is_timeout() || e.is_io_error() {
            SessionError::Backend(e.to_string())
        } else {
            tracing::debug!(error = %e, "session rejected by redis");
            SessionError::Invalid
        }
    }
}

#[async_trait]
impl SessionManager for RedisJwtSessions {
    async fn issue(&self, user_id: UserId) -> Result<TokenPair, SessionError> {
        let now = Utc::now();
        let access_exp = now
            + TimeDelta::from_std(self.config.access_ttl)
                .map_err(|e| SessionError::Backend(e.to_string()))?;
        let refresh_exp = now
            + TimeDelta::from_std(self.config.refresh_ttl)
                .map_err(|e| SessionError::Backend(e.to_string()))?;
        let family_id = Uuid::new_v4();
        let refresh_token = Self::random_refresh(family_id);
        let access_token = self.sign_access(user_id, now, access_exp)?;

        let ttl_secs = self.config.refresh_ttl.as_secs();
        let refresh_key = format!("refresh:{refresh_token}");
        let family_current = format!("family:{}:current", family_id.as_simple());

        let mut conn = self.redis.clone();
        let mut pipe = redis::pipe();
        pipe.atomic()
            .cmd("SET")
            .arg(&refresh_key)
            .arg(user_id.0.to_string())
            .arg("EX")
            .arg(ttl_secs)
            .ignore()
            .cmd("SET")
            .arg(&family_current)
            .arg(&refresh_token)
            .arg("EX")
            .arg(ttl_secs)
            .ignore();
        let _: () = pipe
            .query_async(&mut conn)
            .await
            .map_err(|e| SessionError::Backend(e.to_string()))?;

        Ok(TokenPair {
            access_token,
            refresh_token,
            access_expires_at: access_exp,
            refresh_expires_at: refresh_exp,
        })
    }

    async fn rotate(&self, refresh_token: &str) -> Result<TokenPair, SessionError> {
        let family_id = Self::parse_family(refresh_token)?;
        let now = Utc::now();
        let access_exp = now
            + TimeDelta::from_std(self.config.access_ttl)
                .map_err(|e| SessionError::Backend(e.to_string()))?;
        let refresh_exp = now
            + TimeDelta::from_std(self.config.refresh_ttl)
                .map_err(|e| SessionError::Backend(e.to_string()))?;
        let new_refresh = Self::random_refresh(family_id);
        let ttl_secs = self.config.refresh_ttl.as_secs();

        let refresh_key = format!("refresh:{refresh_token}");
        let family_current = format!("family:{}:current", family_id.as_simple());
        let family_revoked = format!("family:{}:revoked", family_id.as_simple());
        let new_refresh_key = format!("refresh:{new_refresh}");

        let mut conn = self.redis.clone();
        let user_id_str: String = self
            .rotate_script
            .key(&refresh_key)
            .key(&family_current)
            .key(&family_revoked)
            .key(&new_refresh_key)
            .arg(refresh_token)
            .arg(new_refresh.as_str())
            .arg(ttl_secs)
            .invoke_async(&mut conn)
            .await
            .map_err(Self::map_redis)?;

        let user_uuid = Uuid::parse_str(&user_id_str).map_err(|_| SessionError::Invalid)?;
        let user_id = UserId(user_uuid);
        let access_token = self.sign_access(user_id, now, access_exp)?;

        Ok(TokenPair {
            access_token,
            refresh_token: new_refresh,
            access_expires_at: access_exp,
            refresh_expires_at: refresh_exp,
        })
    }

    fn verify_access(&self, access_token: &str) -> Result<UserId, SessionError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.config.jwt_issuer]);
        let data = decode::<AccessClaims>(access_token, &self.decoding, &validation)
            .map_err(|_| SessionError::Invalid)?;
        let uuid = Uuid::parse_str(&data.claims.sub).map_err(|_| SessionError::Invalid)?;
        Ok(UserId(uuid))
    }

    async fn revoke(&self, refresh_token: &str) -> Result<(), SessionError> {
        let mut conn = self.redis.clone();
        let refresh_key = format!("refresh:{refresh_token}");
        let mut pipe = redis::pipe();
        pipe.cmd("DEL").arg(&refresh_key).ignore();
        if let Ok(family_id) = Self::parse_family(refresh_token) {
            let family_current = format!("family:{}:current", family_id.as_simple());
            pipe.cmd("DEL").arg(&family_current).ignore();
        }
        let _: () = pipe
            .query_async(&mut conn)
            .await
            .map_err(|e| SessionError::Backend(e.to_string()))?;
        Ok(())
    }
}
