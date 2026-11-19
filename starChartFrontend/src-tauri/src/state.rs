use star_chart_protocol::{ClientMsgType, ServerMsgType};
use tokio::sync::RwLock;

use crate::{
    error::{AppError, AppResult},
    server::ServerConnection,
};

const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:3000";

/// Everything the app keeps between commands. Tauri holds the one copy (see `lib.rs`)
/// and hands it to any command that asks for `State<'_, AppState>`.
pub struct AppState {
    pub server_url: String,
    pub http: reqwest::Client,
    /// `None` until the user logs in
    connection: RwLock<Option<ServerConnection>>,
}

impl AppState {
    /// Uses `STAR_CHART_SERVER` as the server address if it's set, e.g. `http://192.168.1.10:3000`
    pub fn from_env() -> Self {
        let server_url = std::env::var("STAR_CHART_SERVER")
            .unwrap_or_else(|_| DEFAULT_SERVER_URL.to_string());

        Self {
            server_url,
            http: reqwest::Client::new(),
            connection: RwLock::new(None),
        }
    }

    /// The WebSocket address: `http://` becomes `ws://`, and `https://` becomes `wss://`
    pub fn ws_url(&self) -> String {
        self.server_url.replacen("http", "ws", 1)
    }

    /// Replaces the open connection, and closes the old one if there was one
    pub async fn set_connection(&self, connection: Option<ServerConnection>) {
        let old = std::mem::replace(&mut *self.connection.write().await, connection);
        if let Some(old) = old {
            old.close().await;
        }
    }

    /// Sends a message on the open connection and waits for the reply
    pub async fn request(&self, msg: ClientMsgType) -> AppResult<ServerMsgType> {
        let connection = self.connection.read().await;
        connection
            .as_ref()
            .ok_or(AppError::NotConnected)?
            .request(msg)
            .await
    }
}
