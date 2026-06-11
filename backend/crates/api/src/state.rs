use std::sync::Arc;

use application::auth::{
    google::GoogleAuth, login::Login, logout::Logout, refresh::Refresh, register::Register,
};
use application::ports::SessionManager;
use application::users::{get_user::GetUser, list_users::ListUsers};

#[derive(Clone)]
pub struct AppState {
    pub register: Arc<Register>,
    pub login: Arc<Login>,
    pub refresh: Arc<Refresh>,
    pub logout: Arc<Logout>,
    pub get_user: Arc<GetUser>,
    pub list_users: Arc<ListUsers>,
    pub sessions: Arc<dyn SessionManager>,
    /// None when the Google OAuth env vars are not set — the /auth/google/*
    /// handlers respond 503 in that case.
    pub google_auth: Option<Arc<GoogleAuth>>,
    /// Where /auth/google/callback redirects the browser on success.
    pub google_post_login_redirect: Option<String>,
    /// Where /auth/google/* handlers redirect with `?oauth_error=` on failure.
    pub google_error_redirect: Option<String>,
}
