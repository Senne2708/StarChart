use std::sync::{Arc, Mutex};

use axum::{Router, middleware};
use axum::routing::{get, post};
use tokio::sync::broadcast;
use database::temp_database::Store;
use messaging::protocol::{ServerMsg};

use crate::auth::auth_middleware::auth_middleware;
use crate::auth::jwt_token::JwtKeys;
use crate::auth::login::{create_login_handler, login_handler};
use crate::logging::init_tracing;
use crate::messaging::websocket::websocket_handler;

mod database;
mod messaging;
mod auth;
mod logging;

struct AppState {
    store: Mutex<Store>,
    tx: broadcast::Sender<ServerMsg>,
    jwt: JwtKeys
}


#[tokio::main]
async fn main() {

    let _log_guard = init_tracing();

    tracing::info!("Service Starting");

    let (tx, _) = broadcast::channel(50);
    let state = Arc::new(AppState {
        store: Mutex::new(Store::new()),
        tx,
        jwt: JwtKeys::from_env()
    });

    let private_route = Router::new()
        .route("/ws", get(websocket_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    let public_route = Router::new()
        .route("/login", post(login_handler))
        .route("/create_login", post(create_login_handler));

    let app = Router::new()
        .merge(private_route)
        .merge(public_route)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
