use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::{Router, middleware};
use messaging::protocol::ServerMsg;
use tokio::sync::{Semaphore, broadcast};

use crate::auth::auth_middleware::auth_middleware;
use crate::auth::jwt_token::JwtKeys;
use crate::auth::login::{create_login_handler, get_admin_hash, login_handler};
use crate::logging::init_tracing;
use crate::messaging::websocket::websocket_handler;

mod auth;
mod database;
mod logging;
mod messaging;

struct AppState {
    db: database::StarChartDb,
    tx: broadcast::Sender<ServerMsg>,
    jwt: JwtKeys,
    admin_hash: String,
    hash_limit: Semaphore,
}

#[tokio::main]
async fn main() {
    let _log_guard = init_tracing();

    tracing::info!("Service Starting");

    let db = database::StarChartDb::connect().await;

    let admin_hash = get_admin_hash()
        .await
        .expect("ADMIN_PASSWORD must be set and hashable");

    let (tx, _) = broadcast::channel(50);
    let hash_limit = Semaphore::new(2);

    let state = Arc::new(AppState {
        db,
        tx,
        jwt: JwtKeys::from_env(),
        admin_hash,
        hash_limit,
    });

    let private_route =
        Router::new()
            .route("/ws", get(websocket_handler))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ));

    let public_route = Router::new()
        .route("/login", post(login_handler))
        .route("/create_login", post(create_login_handler))
        .layer(DefaultBodyLimit::max(4 * 1024));

    let app = Router::new()
        .merge(private_route)
        .merge(public_route)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
