use std::sync::Arc;

use crate::ports::{SessionError, SessionManager};

pub struct LogoutInput {
    pub refresh_token: String,
}

pub struct Logout {
    sessions: Arc<dyn SessionManager>,
}

impl Logout {
    pub fn new(sessions: Arc<dyn SessionManager>) -> Self {
        Self { sessions }
    }

    pub async fn execute(&self, input: LogoutInput) -> Result<(), SessionError> {
        self.sessions.revoke(&input.refresh_token).await
    }
}
