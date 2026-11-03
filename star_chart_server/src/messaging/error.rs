use thiserror::Error;

use crate::database::AppDatabaseError;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error(transparent)]
    Database(#[from] AppDatabaseError),

    #[error("{0}")]
    Validation(String),

    #[error("password hashing failed: {0}")]
    Hashing(String),

    #[error("Background task failed")]
    Task(#[from] tokio::task::JoinError),
}

impl ServerError {
    pub fn code(&self) -> &'static str {
        match self {
            ServerError::Validation(_) => "invalid_input",
            ServerError::Database(AppDatabaseError::NotFound { .. }) => "not_found",
            ServerError::Database(AppDatabaseError::Conflict { .. }) => "conflict",
            _ => "internal_error",
        }
    }

    pub fn client_message(&self) -> String {
        match self {
            Self::Validation(msg) => msg.clone(),
            Self::Database(e) => e.client_message(),
            _ => "internal error".to_string(),
        }
    }

    pub fn is_internal(&self) -> bool {
        self.code() == "internal_error"
    }
}

pub type AppResult<T> = Result<T, ServerError>;
