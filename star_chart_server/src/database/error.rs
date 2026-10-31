use axum::{Json, http::StatusCode, response::IntoResponse};

#[derive(Debug)]
pub enum AppDatabaseError {
    NotFound { entity: &'static str, id: String},
    Temp { reason: &'static str },
    Internal(sqlx::Error),
}

impl From<sqlx::Error> for AppDatabaseError {
    fn from(e: sqlx::Error) -> Self {
        AppDatabaseError::Internal(e)
    }
}

impl AppDatabaseError {
    pub fn client_message(&self) -> String {
        match self {
            AppDatabaseError::NotFound { entity, .. } => format!("{entity} not found"),
            AppDatabaseError::Internal(_) => "internal error".into(),
            AppDatabaseError::Temp { reason } => format!("Error Reason: {reason}"),
        }
    }

    pub fn not_found(entity: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound { entity, id: id.into() }
    }
}

impl IntoResponse for AppDatabaseError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            AppDatabaseError::NotFound { .. } => StatusCode::NOT_FOUND,
            AppDatabaseError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppDatabaseError::Temp { .. } => StatusCode::INTERNAL_SERVER_ERROR
        };
        if let AppDatabaseError::Internal(e) = &self {
            tracing::error!(?e, "database error");
        }
        (status, Json(serde_json::json!({ "error": self.client_message() }))).into_response()
    }
}
