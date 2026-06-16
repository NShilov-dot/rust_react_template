//! Session manager: JWT (HS256) access tokens + opaque, rotating refresh tokens.
//!
//! ## Refresh token format (v2)
//!
//! `{family_id_hex}.{rand_b64url}.{mac_b64url}`
//!
//! - `family_id_hex` — 32 hex chars (16-byte UUID in "simple" form). Encodes the
//!   family so we can locate the correct Redis keys without an extra lookup.
//! - `rand_b64url` — 32 random bytes, URL-safe base64 no-pad (43 chars). Provides
//!   256 bits of unpredictability even if the MAC key were somehow known.
//! - `mac_b64url` — HMAC-SHA256 over `{family_id_hex}.{rand_b64url}` (43 chars).
//!   Computed with a key derived from `JWT_SECRET` using domain separation.
//!   **Rejects tokens that don't verify before any Redis operation.** This closes
//!   the audit finding where a caller who learned a `family_id` (e.g. from a log
//!   leak) could present `{victim_family}.garbage` and trigger reuse-detection,
//!   killing the legitimate session.
//!
//! ## Redis keys
//!
//! - `refresh:{token}` → `user_id` (string). TTL = `refresh_ttl`.
//! - `family:{family_id}:current` → currently-valid token in this family.
//!   TTL slides on every rotation.
//! - `family:{family_id}:revoked` → `"1"` if reuse was detected. TTL = `refresh_ttl`.
//! - `family:{family_id}:born` → `"1"`, immutable TTL = `refresh_ttl`. Absolute
//!   ceiling on family age — when this key expires, further rotations are rejected
//!   even if the client keeps the sliding window alive. Forces periodic
//!   re-authentication regardless of activity level.
//!
//! ## Rotation (atomic via Lua)
//!
//! MAC is verified in Rust BEFORE calling the script. The script then:
//! 1. Rejects if `family:revoked` exists.
//! 2. Rejects if `family:born` is gone (absolute expiry hit).
//! 3. Looks up `user_id` from the presented `refresh:{token}`.
//! 4. Compares presented token against `family:current`.
//!    - Match → rotate: swap keys, extend sliding TTLs.
//!    - Mismatch → reuse detected: revoke family, kill both parties.

use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, TimeDelta, Utc};
use hmac::{Hmac, Mac};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use application::ports::{SessionError, SessionManager, TokenPair};
use domain::UserId;

use crate::config::Secret;

type HmacSha256 = Hmac<Sha256>;

/// Audience claim for access tokens. Validated on every `verify_access` call.
/// If this service ever shares `JWT_SECRET` with another service, the other
/// service must use a different audience so tokens are not cross-replayable.
const ACCESS_TOKEN_AUD: &str = "api";

// ─── Lua rotation script ─────────────────────────────────────────────────────
//
// KEYS[1] = refresh:{old_token}
// KEYS[2] = family:{id}:current
// KEYS[3] = family:{id}:revoked
// KEYS[4] = refresh:{new_token}
// KEYS[5] = family:{id}:born
//
// ARGV[1] = old_token  (compared against KEYS[2] for reuse detection)
// ARGV[2] = new_token
// ARGV[3] = ttl_secs   (sliding TTL applied to all mutable keys)
const ROTATE_LUA: &str = r#"
if redis.call('EXISTS', KEYS[3]) == 1 then
    return redis.error_reply('family_revoked')
end
if redis.call('EXISTS', KEYS[5]) == 0 then
    return redis.error_reply('family_expired')
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

// ─── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub jwt_secret: Secret,
    pub jwt_issuer: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

// ─── Internal types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct AccessClaims {
    sub: String,
    iss: String,
    /// Audience — always `"api"`. Prevents cross-service token replay: a JWT
    /// issued by this service cannot be accepted by another service that shares
    /// the same `JWT_SECRET` but validates `aud` against a different value.
    aud: String,
    iat: i64,
    exp: i64,
}

// ─── Session manager ──────────────────────────────────────────────────────────

pub struct RedisJwtSessions {
    redis: ConnectionManager,
    encoding: EncodingKey,
    decoding: DecodingKey,
    config: SessionConfig,
    /// HMAC-SHA256 key derived from `JWT_SECRET` with domain separation.
    /// Used to sign and verify refresh token MACs. Stored as a `Vec<u8>`
    /// (32 bytes) to avoid re-deriving on every token operation.
    hmac_key: Vec<u8>,
    rotate_script: redis::Script,
}

impl RedisJwtSessions {
    pub fn new(redis: ConnectionManager, config: SessionConfig) -> anyhow::Result<Self> {
        if config.jwt_secret.expose().len() < 32 {
            anyhow::bail!("JWT_SECRET must be at least 32 bytes");
        }
        let encoding = EncodingKey::from_secret(config.jwt_secret.expose().as_bytes());
        let decoding = DecodingKey::from_secret(config.jwt_secret.expose().as_bytes());

        // Derive a domain-separated HMAC-SHA256 key via HMAC-PRF.
        // Using HMAC(secret, label) separates the "refresh token MAC" key
        // namespace from the JWT signing key even though both stem from
        // the same source secret — prevents any cross-context key reuse.
        let hmac_key = {
            let mut m = HmacSha256::new_from_slice(config.jwt_secret.expose().as_bytes())
                .expect("HMAC accepts any key length");
            m.update(b"refresh_token_mac:v1");
            m.finalize().into_bytes().to_vec()
        };

        let rotate_script = redis::Script::new(ROTATE_LUA);
        Ok(Self {
            redis,
            encoding,
            decoding,
            config,
            hmac_key,
            rotate_script,
        })
    }

    /// Create a new HMAC-signed refresh token for `family_id`.
    ///
    /// Format: `{family_id_simple}.{rand_b64url}.{mac_b64url}`
    fn new_refresh_token(&self, family_id: Uuid) -> String {
        let mut rng_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut rng_bytes);
        let rand_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rng_bytes);
        let body = format!("{}.{}", family_id.as_simple(), rand_b64);
        let mac = self.mac_bytes(body.as_bytes());
        let mac_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac);
        format!("{body}.{mac_b64}")
    }

    /// Compute HMAC-SHA256 over `data` using the derived refresh-token key.
    fn mac_bytes(&self, data: &[u8]) -> impl AsRef<[u8]> {
        let mut m = HmacSha256::new_from_slice(&self.hmac_key)
            .expect("SHA256 output is always a valid HMAC-SHA256 key length");
        m.update(data);
        m.finalize().into_bytes()
    }

    /// Verify the HMAC suffix of a refresh token and return its `family_id`.
    ///
    /// Rejects tokens with a bad MAC (including old pre-HMAC tokens) before
    /// any Redis operation is attempted. The comparison is constant-time via
    /// `hmac::Mac::verify_slice` (uses `subtle::ConstantTimeEq` internally).
    fn verify_refresh_token(&self, token: &str) -> Result<Uuid, SessionError> {
        // Split off the MAC at the *last* dot: body = "{family}.{rand}", mac = last segment.
        let last_dot = token.rfind('.').ok_or(SessionError::Invalid)?;
        let body = &token[..last_dot];
        let mac_b64 = &token[last_dot + 1..];

        let mac_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(mac_b64)
            .map_err(|_| SessionError::Invalid)?;

        let mut m = HmacSha256::new_from_slice(&self.hmac_key)
            .expect("SHA256 output is always a valid HMAC-SHA256 key length");
        m.update(body.as_bytes());
        m.verify_slice(&mac_bytes)
            .map_err(|_| SessionError::Invalid)?;

        let family_hex = body.split('.').next().ok_or(SessionError::Invalid)?;
        Uuid::parse_str(family_hex).map_err(|_| SessionError::Invalid)
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
            aud: ACCESS_TOKEN_AUD.to_string(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| SessionError::Backend(e.to_string()))
    }

    fn map_redis(e: redis::RedisError) -> SessionError {
        // Connection / IO errors are transient backend failures. Lua
        // `error_reply` strings (family_revoked, not_found, reuse_detected,
        // family_expired) and other Redis errors mean the token is logically
        // invalid — don't leak which case it was to the caller.
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
        let refresh_token = self.new_refresh_token(family_id);
        let access_token = self.sign_access(user_id, now, access_exp)?;

        let ttl_secs = self.config.refresh_ttl.as_secs();
        let refresh_key = format!("refresh:{refresh_token}");
        let family_current = format!("family:{}:current", family_id.as_simple());
        // Immutable absolute-age sentinel. When this key's TTL fires, further
        // rotations are rejected (family_expired) even if the client kept the
        // sliding window alive — enforces periodic re-authentication.
        let family_born = format!("family:{}:born", family_id.as_simple());

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
            .ignore()
            .cmd("SET")
            .arg(&family_born)
            .arg("1")
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
        // MAC check first — rejects forged/garbage tokens before touching Redis.
        let family_id = self.verify_refresh_token(refresh_token)?;

        let now = Utc::now();
        let access_exp = now
            + TimeDelta::from_std(self.config.access_ttl)
                .map_err(|e| SessionError::Backend(e.to_string()))?;
        let refresh_exp = now
            + TimeDelta::from_std(self.config.refresh_ttl)
                .map_err(|e| SessionError::Backend(e.to_string()))?;

        let new_refresh = self.new_refresh_token(family_id);
        let ttl_secs = self.config.refresh_ttl.as_secs();

        let refresh_key = format!("refresh:{refresh_token}");
        let family_current = format!("family:{}:current", family_id.as_simple());
        let family_revoked = format!("family:{}:revoked", family_id.as_simple());
        let new_refresh_key = format!("refresh:{new_refresh}");
        let family_born = format!("family:{}:born", family_id.as_simple());

        let mut conn = self.redis.clone();
        let user_id_str: String = self
            .rotate_script
            .key(&refresh_key)
            .key(&family_current)
            .key(&family_revoked)
            .key(&new_refresh_key)
            .key(&family_born)
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
        validation.set_audience(&[ACCESS_TOKEN_AUD]);
        // Require both `exp` and `aud` to be present and valid. `sub` is
        // enforced structurally (non-Option field); `iss` via set_issuer above.
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        // 30-second clock-skew leeway — tight enough to keep replay windows
        // narrow, generous enough for containers with minor clock drift.
        validation.leeway = 30;
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
        // Best-effort: if MAC is invalid we still deleted the refresh key above;
        // we just can't locate the family keys to clean up.
        if let Ok(family_id) = self.verify_refresh_token(refresh_token) {
            let family_current = format!("family:{}:current", family_id.as_simple());
            let family_born = format!("family:{}:born", family_id.as_simple());
            pipe.cmd("DEL").arg(&family_current).ignore();
            pipe.cmd("DEL").arg(&family_born).ignore();
        }
        let _: () = pipe
            .query_async(&mut conn)
            .await
            .map_err(|e| SessionError::Backend(e.to_string()))?;
        Ok(())
    }
}
