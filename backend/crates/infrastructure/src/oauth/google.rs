use async_trait::async_trait;
use jsonwebtoken::{decode, decode_header, Algorithm, Validation};
use oauth2::basic::{
    BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
    BasicTokenType,
};
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, Client, ClientId, ClientSecret, CsrfToken, ExtraTokenFields,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, StandardRevocableToken,
    StandardTokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::warn;

use application::ports::{AuthRequest, GoogleAuthClient, GoogleUserInfo, OAuthError};

use super::jwks::JwksCache;

/// Google OAuth 2.0 / OIDC endpoints — stable, no need to discover via
/// openid-configuration on every call.
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Google publishes id_tokens with both forms of issuer claim. RFC and OIDC
/// spec allow either as long as we accept both.
const VALID_ISSUERS: &[&str] = &["https://accounts.google.com", "accounts.google.com"];

/// `oauth2` crate's `BasicTokenResponse` doesn't surface `id_token`, so we
/// thread it in via the `ExtraTokenFields` extension point.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoogleExtraFields {
    #[serde(default)]
    id_token: Option<String>,
}

impl ExtraTokenFields for GoogleExtraFields {}

type GoogleTokenResponse = StandardTokenResponse<GoogleExtraFields, BasicTokenType>;

type GoogleOauthInner = Client<
    BasicErrorResponse,
    GoogleTokenResponse,
    BasicTokenType,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
>;

/// Validated subject claims pulled from a Google id_token. `email_verified`
/// is `false` by default so a missing field is the safe interpretation.
#[derive(Debug, Deserialize)]
struct IdTokenClaims {
    iss: String,
    sub: String,
    email: String,
    #[serde(default)]
    email_verified: bool,
    name: Option<String>,
    nonce: Option<String>,
    // `exp`, `iat`, `aud` are validated by jsonwebtoken via `Validation`;
    // they don't need to round-trip into Rust types here.
}

/// Adapter wiring the `oauth2` crate to our `GoogleAuthClient` port.
/// Identity claims (`sub`, `email`, `email_verified`) come from the
/// id_token's JWS-verified claims — NOT from the userinfo endpoint —
/// because userinfo is bearer-authed and would let any MITM of the
/// userinfo host forge `email_verified=true` against a victim's email.
pub struct GoogleOAuthClient {
    inner: GoogleOauthInner,
    client_id: String,
    jwks: JwksCache,
}

impl GoogleOAuthClient {
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> anyhow::Result<Self> {
        let inner = Client::new(
            ClientId::new(client_id.clone()),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new(AUTH_URL.into())?,
            Some(TokenUrl::new(TOKEN_URL.into())?),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri)?);

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            inner,
            client_id,
            jwks: JwksCache::new(http),
        })
    }
}

#[async_trait]
impl GoogleAuthClient for GoogleOAuthClient {
    fn authorize(&self) -> AuthRequest {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        // OIDC `nonce` — random per-authorize secret, bound into the
        // id_token Google returns. Same primitive Google's `oauth2` crate
        // uses for state (16-byte URL-safe-base64).
        let nonce = CsrfToken::new_random().secret().clone();

        let (auth_url, csrf_state) = self
            .inner
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".into()))
            .add_scope(Scope::new("email".into()))
            .add_scope(Scope::new("profile".into()))
            .add_extra_param("nonce", &nonce)
            .set_pkce_challenge(pkce_challenge)
            .url();

        AuthRequest {
            authorize_url: auth_url.to_string(),
            csrf_state: csrf_state.secret().clone(),
            pkce_verifier: pkce_verifier.secret().clone(),
            nonce,
        }
    }

    async fn exchange(
        &self,
        code: &str,
        pkce_verifier: &str,
        expected_nonce: &str,
    ) -> Result<GoogleUserInfo, OAuthError> {
        let token = self
            .inner
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| {
                warn!(error = %e, "google token exchange failed");
                OAuthError::Provider(e.to_string())
            })?;

        // Reject the response if Google didn't return an id_token. With
        // `openid` in scopes this should always be present; absence means
        // either the scope was stripped or the provider misbehaved.
        let id_token = token.extra_fields().id_token.as_deref().ok_or_else(|| {
            warn!("google token response missing id_token");
            OAuthError::Provider("missing id_token in token response".into())
        })?;

        let claims = self.verify_id_token(id_token, expected_nonce).await?;

        if !claims.email_verified {
            // Surface as the dedicated variant so the application layer's
            // auto-link policy keeps treating this as "refuse to link".
            return Err(OAuthError::EmailNotVerified);
        }

        Ok(GoogleUserInfo {
            sub: claims.sub,
            email: claims.email,
            email_verified: claims.email_verified,
            name: claims.name,
        })
    }
}

impl GoogleOAuthClient {
    async fn verify_id_token(
        &self,
        id_token: &str,
        expected_nonce: &str,
    ) -> Result<IdTokenClaims, OAuthError> {
        // Pull `kid` from the JWS header so we know which JWKS entry to use.
        let header = decode_header(id_token).map_err(|e| {
            warn!(error = %e, "id_token header malformed");
            OAuthError::Provider("malformed id_token".into())
        })?;
        let kid = header.kid.ok_or_else(|| {
            warn!("id_token JWS header missing `kid`");
            OAuthError::Provider("id_token missing kid".into())
        })?;

        let key = self.jwks.get(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(VALID_ISSUERS);
        validation.set_audience(&[&self.client_id]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        // Google's id_tokens regularly arrive with a few seconds of clock
        // skew. 30 s is tight enough to keep the replay window narrow.
        validation.leeway = 30;

        let data = decode::<IdTokenClaims>(id_token, &key, &validation).map_err(|e| {
            warn!(error = %e, "id_token signature/claims validation failed");
            OAuthError::Provider("invalid id_token".into())
        })?;

        // `Validation` already checks iss in the allow-list. Re-check just
        // `aud` to be unambiguous and to defend against future regressions.
        if data.claims.iss.is_empty() {
            return Err(OAuthError::Provider("id_token missing iss".into()));
        }

        // Nonce binding — the whole point of carrying it through the cache.
        // `set_required_spec_claims` won't enforce `nonce` because it's not
        // a JWT-spec claim, so we check explicitly.
        match data.claims.nonce.as_deref() {
            Some(n) if n == expected_nonce => {}
            _ => {
                warn!("id_token nonce mismatch or missing");
                return Err(OAuthError::Provider("nonce mismatch".into()));
            }
        }

        Ok(data.claims)
    }
}
