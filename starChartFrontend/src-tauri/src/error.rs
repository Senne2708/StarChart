use serde::{Serialize, Serializer};
use star_chart_protocol::ErrorCode;
use tokio_tungstenite::tungstenite;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not logged in")]
    NotConnected,

    #[error("wrong username or password")]
    BadLogin,

    #[error("wrong admin code")]
    BadAdminCode,

    #[error("that username is already taken")]
    UsernameTaken,

    #[error("username must be 1-50 characters and password 8-128 characters")]
    InvalidAccount,

    #[error("could not reach the server: {0}")]
    Http(#[from] reqwest::Error),

    #[error("connection error: {0}")]
    WebSocket(Box<tungstenite::Error>),

    #[error("the connection to the server closed")]
    Disconnected,

    #[error("the server took too long to reply")]
    Timeout,

    /// The server replied with an `error` message
    #[error("{message}")]
    Server { code: ErrorCode, message: String },

    #[error("unexpected reply from the server")]
    UnexpectedReply,
}

// Boxed, because the tungstenite error is large and every `Result` would pay for it
impl From<tungstenite::Error> for AppError {
    fn from(e: tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(e))
    }
}

impl AppError {
    fn kind(&self) -> &'static str {
        match self {
            Self::NotConnected => "not_connected",
            Self::BadLogin => "bad_login",
            Self::BadAdminCode => "bad_admin_code",
            Self::UsernameTaken => "username_taken",
            Self::InvalidAccount => "invalid_account",
            Self::Http(_) => "http",
            Self::WebSocket(_) => "websocket",
            Self::Disconnected => "disconnected",
            Self::Timeout => "timeout",
            Self::Server { .. } => "server",
            Self::UnexpectedReply => "unexpected_reply",
        }
    }
}

/// What the React UI receives when a command fails
#[derive(Serialize)]
struct ErrorPayload<'a> {
    kind: &'static str,
    /// Only set for `server` errors, e.g. `invalid_input`
    code: Option<&'a ErrorCode>,
    message: String,
}

// Tauri sends command errors to the UI as JSON, so they must be serializable
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let code = match self {
            Self::Server { code, .. } => Some(code),
            _ => None,
        };
        ErrorPayload {
            kind: self.kind(),
            code,
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

pub type AppResult<T> = Result<T, AppError>;
