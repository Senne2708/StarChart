use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use star_chart_protocol::{ClientMsg, ClientMsgType, ServerMsg, ServerMsgType};
use tauri::{AppHandle, Emitter, async_runtime::JoinHandle};
use tokio::{net::TcpStream, sync::oneshot};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{
        Message,
        client::IntoClientRequest,
        http::{HeaderValue, header::AUTHORIZATION},
    },
};

use crate::error::{AppError, AppResult};

/// Event the React UI listens for: every broadcast (`student_added`, `activity_added`,
/// `activity_removed`, `student_removed`, `resync`)
pub const SERVER_EVENT: &str = "server-event";
/// Event sent when the connection drops without the user logging out
pub const DISCONNECTED_EVENT: &str = "server-disconnected";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Requests waiting for a reply, by message id
type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<ServerMsgType>>>>;

/// An open WebSocket to the server.
///
/// Sending happens in `request`. A background task reads everything the server sends:
/// replies go to the request waiting for them, broadcasts go to the UI as events.
pub struct ServerConnection {
    writer: tokio::sync::Mutex<SplitSink<Socket, Message>>,
    pending: Pending,
    next_id: AtomicU64,
    read_task: JoinHandle<()>,
}

impl ServerConnection {
    pub async fn connect(ws_url: &str, token: &str, app: AppHandle) -> AppResult<Self> {
        let mut request = format!("{ws_url}/ws").into_client_request()?;
        let auth = HeaderValue::from_str(&format!("Bearer {token}"))
            .expect("a JWT is always a valid header value");
        request.headers_mut().insert(AUTHORIZATION, auth);

        let (socket, _) = tokio_tungstenite::connect_async(request).await?;
        let (writer, reader) = socket.split();

        let pending = Pending::default();
        let read_task = tauri::async_runtime::spawn(read_loop(reader, pending.clone(), app));

        Ok(Self {
            writer: tokio::sync::Mutex::new(writer),
            pending,
            next_id: AtomicU64::new(1),
            read_task,
        })
    }

    /// Sends `msg` and waits for the server's reply to it
    pub async fn request(&self, msg: ClientMsgType) -> AppResult<ServerMsgType> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Register before sending, so a fast reply can't arrive before we're listening
        let (reply_tx, reply_rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, reply_tx);

        let text = serde_json::to_string(&ClientMsg { id, msg }).expect("ClientMsg should always serialize");
        if let Err(e) = self.writer.lock().await.send(Message::text(text)).await {
            self.pending.lock().unwrap().remove(&id);
            return Err(e.into());
        }

        let reply = match tokio::time::timeout(REQUEST_TIMEOUT, reply_rx).await {
            Ok(Ok(reply)) => reply,
            // The read task dropped the sender, so the connection closed
            Ok(Err(_)) => return Err(AppError::Disconnected),
            Err(_) => {
                self.pending.lock().unwrap().remove(&id);
                return Err(AppError::Timeout);
            }
        };

        match reply {
            ServerMsgType::Error { code, message } => Err(AppError::Server { code, message }),
            other => Ok(other),
        }
    }

    /// Closes the connection on purpose, without sending `server-disconnected`
    pub async fn close(self) {
        self.read_task.abort();
        let _ = self.writer.lock().await.close().await;
    }
}

impl Drop for ServerConnection {
    fn drop(&mut self) {
        self.read_task.abort();
    }
}

async fn read_loop(mut reader: SplitStream<Socket>, pending: Pending, app: AppHandle) {
    while let Some(frame) = reader.next().await {
        let text = match frame {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) | Err(_) => break,
            // Pings are answered by tungstenite, the server sends nothing else
            Ok(_) => continue,
        };

        let msg: ServerMsg = match serde_json::from_str(text.as_str()) {
            Ok(msg) => msg,
            Err(e) => {
                tracing::warn!("unreadable message from server: {e}");
                continue;
            }
        };

        match msg.id {
            // A reply: hand it to the request waiting for it
            Some(id) => {
                if let Some(reply_tx) = pending.lock().unwrap().remove(&id) {
                    let _ = reply_tx.send(msg.msg);
                }
            }
            // A broadcast: pass it on to the UI
            None => {
                let _ = app.emit(SERVER_EVENT, msg.msg);
            }
        }
    }

    // Dropping the senders makes every waiting request fail with `Disconnected`
    pending.lock().unwrap().clear();
    let _ = app.emit(DISCONNECTED_EVENT, ());
}
