use std::{net::SocketAddr, sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::{
    net::TcpStream,
    sync::{Semaphore, broadcast, watch},
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{
        self, Message,
        client::IntoClientRequest,
        http::{HeaderValue, header::AUTHORIZATION},
    },
};

use crate::{
    AppState, app,
    auth::{
        jwt_token::{JwtKeys, create_jwt},
        password::hash_password,
        rate_limit::{LoginLimits, RateLimiter},
    },
    database::StarChartDb,
};

pub const ADMIN_CODE: &str = "test-admin-code";

/// Limits high enough that tests never hit them. Tests about rate limiting
/// replace the one they check.
pub fn generous_limits() -> LoginLimits {
    LoginLimits {
        per_ip: RateLimiter::new(1000, Duration::from_secs(60)),
        password_failures: RateLimiter::new(1000, Duration::from_secs(60)),
        admin_code_failures: RateLimiter::new(1000, Duration::from_secs(60)),
    }
}

/// A running server with a fresh database
pub struct TestServer {
    pub addr: SocketAddr,
    pub state: Arc<AppState>,
    pub shutdown_tx: watch::Sender<()>,
    http: reqwest::Client,
}

impl TestServer {
    pub async fn start() -> Self {
        Self::start_with_limits(generous_limits()).await
    }

    pub async fn start_with_limits(login_limits: LoginLimits) -> Self {
        let (tx, _) = broadcast::channel(64);
        let (shutdown_tx, shutdown) = watch::channel(());

        let state = Arc::new(AppState {
            db: StarChartDb::in_memory().await,
            tx,
            jwt: JwtKeys::from_secret(b"test-secret-that-is-at-least-32-chars"),
            admin_hash: hash_password(ADMIN_CODE).unwrap(),
            hash_limit: Semaphore::new(1),
            login_limits,
            shutdown,
        });

        // Port 0 lets the OS pick a free port, so tests can run at the same time
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let service = app(state.clone()).into_make_service_with_connect_info::<SocketAddr>();
        tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });

        // Ignore any proxy set in the environment, the server is on localhost
        let http = reqwest::Client::builder().no_proxy().build().unwrap();

        Self {
            addr,
            state,
            shutdown_tx,
            http,
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    pub async fn post_json(&self, path: &str, body: Value) -> reqwest::Response {
        self.http
            .post(self.url(path))
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    /// Posts JSON as if through a reverse proxy, for a client at `client_ip`
    pub async fn post_json_forwarded_for(
        &self,
        path: &str,
        body: Value,
        client_ip: &str,
    ) -> reqwest::Response {
        self.http
            .post(self.url(path))
            .header("x-forwarded-for", client_ip)
            .json(&body)
            .send()
            .await
            .unwrap()
    }

    /// Posts `body` exactly as given, with an optional `Content-Type`
    pub async fn post_raw(
        &self,
        path: &str,
        content_type: Option<&str>,
        body: &str,
    ) -> reqwest::Response {
        let mut request = self.http.post(self.url(path)).body(body.to_string());
        if let Some(content_type) = content_type {
            request = request.header("content-type", content_type);
        }
        request.send().await.unwrap()
    }

    /// Adds a teacher straight to the database, skipping the slow password hashing,
    /// and returns its id and a valid token for it
    pub async fn teacher(&self, name: &str) -> (i64, String) {
        let teacher = self
            .state
            .db
            .teachers
            .create(name, "unused-hash")
            .await
            .unwrap();
        let token = create_jwt(&self.state.jwt, name).unwrap();
        (teacher.id, token)
    }

    fn ws_request(&self, token: Option<&str>) -> tungstenite::handshake::client::Request {
        let mut request = format!("ws://{}/ws", self.addr)
            .into_client_request()
            .unwrap();
        if let Some(token) = token {
            let value = HeaderValue::from_str(&format!("Bearer {token}")).unwrap();
            request.headers_mut().insert(AUTHORIZATION, value);
        }
        request
    }

    /// Opens a WebSocket and waits until the server is ready for it
    pub async fn connect(&self, token: &str) -> WsClient {
        let (stream, _) = tokio_tungstenite::connect_async(self.ws_request(Some(token)))
            .await
            .expect("websocket should connect");
        let mut client = WsClient { stream, next_id: 0 };

        // The server subscribes to broadcasts as soon as the socket starts, so once
        // one request has been answered, this client is sure to get broadcasts
        client.request(json!({ "type": "get_teachers" })).await;
        client
    }

    /// Tries to open a WebSocket that should be refused, and returns the HTTP status
    pub async fn ws_rejection_status(&self, token: Option<&str>) -> u16 {
        match tokio_tungstenite::connect_async(self.ws_request(token)).await {
            Err(tungstenite::Error::Http(response)) => response.status().as_u16(),
            Err(other) => panic!("unexpected websocket error: {other}"),
            Ok(_) => panic!("websocket should have been refused"),
        }
    }
}

pub struct WsClient {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_id: u64,
}

impl WsClient {
    /// Sends a text frame exactly as given
    pub async fn send_text(&mut self, text: &str) {
        self.stream
            .send(Message::text(text.to_string()))
            .await
            .unwrap();
    }

    /// Wraps `msg` in an envelope with a new id, sends it, and returns the id
    pub async fn send(&mut self, msg: Value) -> u64 {
        self.next_id += 1;
        let envelope = json!({ "id": self.next_id, "msg": msg });
        self.send_text(&envelope.to_string()).await;
        self.next_id
    }

    /// The next frame from the server. Fails the test after 5 seconds.
    pub async fn recv(&mut self) -> Message {
        tokio::time::timeout(Duration::from_secs(5), self.stream.next())
            .await
            .expect("timed out waiting for the server")
            .expect("socket closed")
            .expect("websocket error")
    }

    /// The next message from the server, as JSON with its envelope
    pub async fn recv_json(&mut self) -> Value {
        let frame = self.recv().await;
        serde_json::from_str(frame.to_text().expect("expected a text frame")).unwrap()
    }

    /// Sends `msg`, checks the reply echoes its id, and returns the reply's `msg`
    pub async fn request(&mut self, msg: Value) -> Value {
        let id = self.send(msg).await;
        let reply = self.recv_json().await;
        assert_eq!(reply["id"], id, "the reply should echo the request id");
        reply["msg"].clone()
    }
}
