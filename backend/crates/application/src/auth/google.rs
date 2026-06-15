use std::sync::Arc;

use thiserror::Error;
use tracing::warn;

use domain::{DomainError, Email, User};

use crate::ports::{
    AuthRequest, CacheError, CacheStore, GoogleAuthClient, GoogleUserInfo, OAuthError, RepoError,
    SessionError, SessionManager, TokenPair, UserRepository,
};

#[derive(Debug, Error)]
pub enum GoogleAuthError {
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Cache(#[from] CacheError),
    #[error(transparent)]
    OAuth(#[from] OAuthError),
    #[error(transparent)]
    Session(#[from] SessionError),
    /// Email already used by a local-password user and `email_verified` is
    /// false on the Google side, so we won't auto-link. Treat this as a
    /// hard failure — surface a generic error to the browser; never reveal
    /// "account exists" to a remote caller.
    #[error("cannot link unverified Google email to an existing account")]
    LinkRefused,
}

pub struct GoogleAuthOutput {
    pub user: User,
    pub tokens: TokenPair,
}

/// Owns the policy for "given a verified Google user, who is this in our
/// system?" — keeps the HTTP handler thin.
///
/// Three branches:
///   1. We've seen this `google_id` before → log them in.
///   2. New `google_id`, but their `email` already exists locally AND Google
///      vouches the address is verified → link the Google id to that user.
///      (Verified-only prevents account takeover via attacker-controlled
///      addresses on free providers that don't enforce ownership.)
///   3. Otherwise → create a brand-new OAuth-only user (NULL password_hash).
pub struct GoogleAuth {
    repo: Arc<dyn UserRepository>,
    cache: Arc<dyn CacheStore>,
    client: Arc<dyn GoogleAuthClient>,
    sessions: Arc<dyn SessionManager>,
    state_ttl_secs: u64,
}

impl GoogleAuth {
    pub fn new(
        repo: Arc<dyn UserRepository>,
        cache: Arc<dyn CacheStore>,
        client: Arc<dyn GoogleAuthClient>,
        sessions: Arc<dyn SessionManager>,
    ) -> Self {
        Self {
            repo,
            cache,
            client,
            sessions,
            // 5 minutes is the standard tradeoff: enough for slow users to
            // complete the consent screen, short enough that a stolen state
            // token isn't useful long.
            state_ttl_secs: 300,
        }
    }

    /// Step 1 — build the URL we'll bounce the user to, AND persist the
    /// PKCE verifier under the state key so we can recover it in `callback`.
    pub async fn start(&self) -> Result<String, GoogleAuthError> {
        let AuthRequest {
            authorize_url,
            csrf_state,
            pkce_verifier,
        } = self.client.authorize();

        self.cache
            .set_bytes(
                &state_cache_key(&csrf_state),
                pkce_verifier.as_bytes(),
                Some(self.state_ttl_secs),
            )
            .await?;

        Ok(authorize_url)
    }

    /// Step 2 — Google bounced the user back with `code` + `state`. Verify
    /// the state was one we issued (and not replayed), recover the PKCE
    /// verifier, swap the code for user info, then resolve to a session.
    pub async fn callback(
        &self,
        code: &str,
        state: &str,
    ) -> Result<GoogleAuthOutput, GoogleAuthError> {
        let key = state_cache_key(state);
        let Some(pkce_bytes) = self.cache.get_bytes(&key).await? else {
            return Err(GoogleAuthError::OAuth(OAuthError::InvalidState));
        };
        // One-shot: prevent replay even within the TTL window.
        let _ = self.cache.delete(&key).await;

        let pkce_verifier = std::str::from_utf8(&pkce_bytes)
            .map_err(|_| GoogleAuthError::OAuth(OAuthError::InvalidState))?;

        let info = self.client.exchange(code, pkce_verifier).await?;
        let user = self.resolve_user(&info).await?;
        let tokens = self.sessions.issue(user.id).await?;

        Ok(GoogleAuthOutput { user, tokens })
    }

    async fn resolve_user(&self, info: &GoogleUserInfo) -> Result<User, GoogleAuthError> {
        // Branch 1: already-linked Google identity.
        if let Some(user) = self.repo.find_by_google_id(&info.sub).await? {
            return Ok(user);
        }

        // Branch 2: auto-link by verified email.
        let email = Email::parse(&info.email).map_err(|e| {
            warn!(error = %e, "google returned malformed email");
            GoogleAuthError::Domain(e)
        })?;

        if let Some(existing) = self.repo.find_by_email(&email).await? {
            if !info.email_verified {
                // Don't silently merge — that's the takeover vector.
                return Err(GoogleAuthError::LinkRefused);
            }
            self.repo.link_google(existing.id, &info.sub).await?;
            return Ok(existing);
        }

        // Branch 3: brand-new OAuth user.
        let name = info
            .name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| email.as_str().split('@').next().unwrap_or("user").to_string());

        let user = User::new(email, name)?;
        self.repo.create_oauth(&user, &info.sub).await?;
        Ok(user)
    }
}

fn state_cache_key(state: &str) -> String {
    format!("oauth:google:state:{state}")
}
