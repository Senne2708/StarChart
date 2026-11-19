use thiserror::Error;

use crate::{database::AppDatabaseError, messaging::protocol::ErrorCode};

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
    pub fn code(&self) -> ErrorCode {
        match self {
            ServerError::Validation(_) => ErrorCode::InvalidInput,
            ServerError::Database(AppDatabaseError::NotFound { .. }) => ErrorCode::NotFound,
            ServerError::Database(AppDatabaseError::Conflict { .. }) => ErrorCode::Conflict,
            _ => ErrorCode::InternalError,
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
        self.code() == ErrorCode::InternalError
    }
}

pub type AppResult<T> = Result<T, ServerError>;
