use std::sync::Arc;

use application::auth::{
    login::Login, logout::Logout, refresh::Refresh, register::Register,
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
}
