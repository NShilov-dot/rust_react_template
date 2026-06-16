//! Google JWKS fetcher with TTL cache.
//!
//! Google rotates the signing keys for OIDC id_tokens every few weeks and
//! publishes the active set at <https://www.googleapis.com/oauth2/v3/certs>.
//! We fetch on demand, key by `kid`, and cache for `CACHE_TTL`. A cache miss
//! after that window triggers a refetch under the write lock — concurrent
//! callbacks coalesce on the same fetch.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use jsonwebtoken::DecodingKey;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::warn;

use application::ports::OAuthError;

/// Google's published JWKS URL. Stable — no need to discover via
/// openid-configuration on every call.
const JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";

/// How long a fetched JWKS is reused before refetching. 1 h is well under
/// Google's typical 6h+ rotation cadence; an old `kid` that vanishes
/// before our cache expires triggers a synchronous refetch on lookup.
const CACHE_TTL: Duration = Duration::from_secs(3600);

#[derive(Debug, Deserialize)]
struct JwksDoc {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    alg: Option<String>,
    n: String,
    e: String,
}

struct CacheState {
    fetched_at: Instant,
    keys: HashMap<String, DecodingKey>,
}

pub struct JwksCache {
    http: reqwest::Client,
    state: RwLock<Option<CacheState>>,
}

impl JwksCache {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            http,
            state: RwLock::new(None),
        }
    }

    /// Look up the decoding key for `kid`. Refetches on TTL expiry or on
    /// a miss for an unknown `kid` (e.g. Google rotated mid-cache-window).
    pub async fn get(&self, kid: &str) -> Result<DecodingKey, OAuthError> {
        // Fast path: cached and fresh.
        {
            let guard = self.state.read().await;
            if let Some(state) = guard.as_ref() {
                if state.fetched_at.elapsed() < CACHE_TTL {
                    if let Some(key) = state.keys.get(kid) {
                        return Ok(key.clone());
                    }
                }
            }
        }

        // Slow path: refetch under write lock. A second concurrent caller
        // racing us will re-check the cache after we drop the lock.
        let mut guard = self.state.write().await;
        // Re-check after acquiring write lock — another task may have
        // refetched while we were waiting.
        if let Some(state) = guard.as_ref() {
            if state.fetched_at.elapsed() < CACHE_TTL {
                if let Some(key) = state.keys.get(kid) {
                    return Ok(key.clone());
                }
            }
        }
        let fresh = fetch(&self.http).await?;
        let key = fresh.keys.get(kid).cloned();
        *guard = Some(fresh);
        key.ok_or_else(|| {
            warn!(kid, "id_token references unknown JWKS kid after refetch");
            OAuthError::Provider("id_token signed by unknown key".into())
        })
    }
}

async fn fetch(http: &reqwest::Client) -> Result<CacheState, OAuthError> {
    let doc: JwksDoc = http
        .get(JWKS_URL)
        .send()
        .await
        .map_err(|e| OAuthError::Network(e.to_string()))?
        .error_for_status()
        .map_err(|e| OAuthError::Provider(e.to_string()))?
        .json()
        .await
        .map_err(|e| OAuthError::Provider(e.to_string()))?;

    let mut keys = HashMap::with_capacity(doc.keys.len());
    for jwk in doc.keys {
        if jwk.kty != "RSA" {
            // Google currently only publishes RSA keys; ignore anything
            // else rather than failing the whole fetch.
            continue;
        }
        if let Some(alg) = jwk.alg.as_deref() {
            if alg != "RS256" {
                continue;
            }
        }
        match DecodingKey::from_rsa_components(&jwk.n, &jwk.e) {
            Ok(k) => {
                keys.insert(jwk.kid, k);
            }
            Err(e) => {
                warn!(kid = %jwk.kid, error = %e, "skipping malformed JWK");
            }
        }
    }

    if keys.is_empty() {
        return Err(OAuthError::Provider("JWKS contained no usable keys".into()));
    }

    Ok(CacheState {
        fetched_at: Instant::now(),
        keys,
    })
}
