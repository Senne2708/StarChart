use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::{Router, middleware};
use messaging::protocol::ServerMsg;
use tokio::sync::{Semaphore, broadcast, watch};

use crate::auth::auth_middleware::auth_middleware;
use crate::auth::jwt_token::JwtKeys;
use crate::auth::login::{create_login_handler, get_admin_hash, init_dummy_hash, login_handler};
use crate::auth::rate_limit::{RateLimiter, login_rate_limit};
use crate::logging::init_tracing;
use crate::messaging::websocket::websocket_handler;

mod auth;
mod database;
mod logging;
mod messaging;
#[cfg(test)]
mod tests;

struct AppState {
    db: database::StarChartDb,
    tx: broadcast::Sender<ServerMsg>,
    jwt: JwtKeys,
    admin_hash: String,
    hash_limit: Semaphore,
    login_limit: RateLimiter,
    shutdown: watch::Receiver<()>,
}

/// `/login` and `/create_login` attempts each IP address gets in a burst.
/// They refill one at a time over a minute, so 5 means one every 12 seconds.
const LOGIN_ATTEMPTS_PER_MINUTE: u32 = 5;

/// Password hashes allowed to run at once. Each one uses ~19 MiB of RAM and a
/// full core, and the server runs on a single core, so more would only queue.
const MAX_CONCURRENT_HASHES: usize = 1;

/// Waits for Ctrl-C or SIGTERM, then tells open WebSockets to close
async fn shutdown_signal(shutdown_tx: watch::Sender<()>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for Ctrl-C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }

    tracing::info!("Shutting down");
    let _ = shutdown_tx.send(());
}

/// Builds the router with every route. The tests start the server through this too.
fn app(state: Arc<AppState>) -> Router {
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
        .layer(DefaultBodyLimit::max(4 * 1024))
        // Added last so it runs first: a refused request never reaches the body or the hashing
        .layer(middleware::from_fn_with_state(
            state.clone(),
            login_rate_limit,
        ));

    Router::new()
        .merge(private_route)
        .merge(public_route)
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let _log_guard = init_tracing();

    tracing::info!("Service Starting");

    let db = database::StarChartDb::connect().await;

    let admin_hash = get_admin_hash()
        .await
        .expect("ADMIN_PASSWORD must be set and hashable");
    init_dummy_hash().await;

    let (tx, _) = broadcast::channel(64);
    let hash_limit = Semaphore::new(MAX_CONCURRENT_HASHES);
    let login_limit = RateLimiter::new(LOGIN_ATTEMPTS_PER_MINUTE, Duration::from_secs(60));
    let (shutdown_tx, shutdown) = watch::channel(());

    let state = Arc::new(AppState {
        db,
        tx,
        jwt: JwtKeys::from_env(),
        admin_hash,
        hash_limit,
        login_limit,
        shutdown,
    });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind 127.0.0.1:3000");

    // `with_connect_info` gives handlers the client's address, which the rate limit needs.
    // Returning from main drops the log guard, which flushes the log file.
    let service = app(state).into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, service)
        .with_graceful_shutdown(shutdown_signal(shutdown_tx))
        .await
        .expect("server error");
}
