use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppDatabaseError {
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("{entity} already exists")]
    Conflict { entity: &'static str },

    #[error("database error: {0}")]
    Internal(#[source] sqlx::Error),
}

impl AppDatabaseError {
    pub fn not_found(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            id: id.into(),
        }
    }

    pub fn client_message(&self) -> String {
        match self {
            Self::NotFound { entity, id } => format!("{entity} {id} not found"),
            Self::Conflict { entity } => format!("{entity} already exists"),
            Self::Internal(_) => "internal error".to_string(),
        }
    }

    pub fn conflict_as(self, entity: &'static str) -> Self {
        match self {
            Self::Conflict { .. } => Self::Conflict { entity },
            other => other,
        }
    }
}

impl From<sqlx::Error> for AppDatabaseError {
    fn from(e: sqlx::Error) -> Self {
        if let Some(db) = e.as_database_error() {
            if db.is_unique_violation() {
                return Self::Conflict { entity: "record" };
            }
        }
        Self::Internal(e)
    }
}
