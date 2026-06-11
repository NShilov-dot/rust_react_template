use async_trait::async_trait;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use std::time::Duration;
use tracing::warn;

use application::ports::{AuthRequest, GoogleAuthClient, GoogleUserInfo, OAuthError};

/// Google OAuth 2.0 endpoints — stable, no need to discover via openid-config.
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";

/// Adapter wiring the `oauth2` crate to our `GoogleAuthClient` port. Also
/// owns a small `reqwest::Client` to fetch the userinfo endpoint after the
/// token exchange.
pub struct GoogleOAuthClient {
    inner: BasicClient,
    http: reqwest::Client,
}

impl GoogleOAuthClient {
    pub fn new(
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> anyhow::Result<Self> {
        let inner = BasicClient::new(
            ClientId::new(client_id),
            Some(ClientSecret::new(client_secret)),
            AuthUrl::new(AUTH_URL.into())?,
            Some(TokenUrl::new(TOKEN_URL.into())?),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri)?);

        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self { inner, http })
    }
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    sub: String,
    email: String,
    /// Google sometimes serializes this as a JSON bool, sometimes as a
    /// string — `serde_with` would be nicer but we own the deserialization
    /// here. Default to false so missing == unverified == safe.
    #[serde(default)]
    email_verified: bool,
    name: Option<String>,
}

#[async_trait]
impl GoogleAuthClient for GoogleOAuthClient {
    fn authorize(&self) -> AuthRequest {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_state) = self
            .inner
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".into()))
            .add_scope(Scope::new("email".into()))
            .add_scope(Scope::new("profile".into()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        AuthRequest {
            authorize_url: auth_url.to_string(),
            csrf_state: csrf_state.secret().clone(),
            pkce_verifier: pkce_verifier.secret().clone(),
        }
    }

    async fn exchange(
        &self,
        code: &str,
        pkce_verifier: &str,
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

        let access_token = token.access_token().secret();

        let info: UserInfoResponse = self
            .http
            .get(USERINFO_URL)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| OAuthError::Network(e.to_string()))?
            .error_for_status()
            .map_err(|e| OAuthError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| OAuthError::Provider(e.to_string()))?;

        Ok(GoogleUserInfo {
            sub: info.sub,
            email: info.email,
            email_verified: info.email_verified,
            name: info.name,
        })
    }
}
