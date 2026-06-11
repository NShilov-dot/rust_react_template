use std::sync::Arc;

use crate::ports::{SessionError, SessionManager, TokenPair};

pub struct RefreshInput {
    pub refresh_token: String,
}

pub struct Refresh {
    sessions: Arc<dyn SessionManager>,
}

impl Refresh {
    pub fn new(sessions: Arc<dyn SessionManager>) -> Self {
        Self { sessions }
    }

    pub async fn execute(&self, input: RefreshInput) -> Result<TokenPair, SessionError> {
        self.sessions.rotate(&input.refresh_token).await
    }
}
