use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    auth::{
        jwt_token::create_jwt,
        password::{hash_password, verify_password},
        rate_limit::too_many_requests,
    },
    database::{AppDatabaseError, db_error},
    messaging::{
        error::{AppResult, ServerError},
        validation::required_text,
    },
};

#[derive(Deserialize)]
pub struct LoginPayload {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
}

#[derive(Deserialize)]
pub struct CreateLoginPayload {
    username: String,
    password: String,
    admin_code: String,
}

/// A failed `/login` or `/create_login`: a plain status, or a 429 that says how long to wait
pub enum AuthError {
    Status(StatusCode),
    TooManyRequests(Duration),
}

impl From<StatusCode> for AuthError {
    fn from(status: StatusCode) -> Self {
        Self::Status(status)
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            Self::Status(status) => status.into_response(),
            Self::TooManyRequests(wait) => too_many_requests(wait),
        }
    }
}

static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_password("dummy-password").expect("failed to hash dummy password"));

/// Builds the dummy hash at startup, so the first unknown-username login doesn't
/// hash on the async runtime or take longer than the others
pub async fn init_dummy_hash() {
    tokio::task::spawn_blocking(|| LazyLock::force(&DUMMY_HASH))
        .await
        .expect("failed to hash dummy password");
}

pub async fn get_admin_hash() -> AppResult<String> {
    let admin_password = match std::env::var("ADMIN_PASSWORD") {
        Ok(password) => password,

        // Only compiled into debug builds
        #[cfg(debug_assertions)]
        Err(_) => {
            tracing::warn!("ADMIN_PASSWORD not set, using insecure debug key");
            "debug_password".to_string()
        }

        // Only compiled into release builds
        #[cfg(not(debug_assertions))]
        Err(_) => panic!("ADMIN_PASSWORD must be set in release builds"),
    };

    tokio::task::spawn_blocking(move || hash_password(&admin_password))
        .await
        .map_err(ServerError::Task)?
        .map_err(|e| ServerError::Hashing(e.to_string()))
}

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<LoginResponse>, AuthError> {
    // Usernames are stored trimmed
    let username = payload.username.trim();

    // Every attempt counts as a failure until the password turns out to be right.
    // Counting before checking means guesses sent in parallel can't get past the limit.
    // Unknown usernames are counted too, so a 429 doesn't reveal which ones exist.
    let failures = &state.login_limits.password_failures;
    failures
        .check(username.to_string())
        .map_err(AuthError::TooManyRequests)?;

    let user = state
        .db
        .teachers
        .get_password_hash(username)
        .await
        .map_err(db_error)?;

    let user_exists = user.is_some();
    let hash = user.unwrap_or_else(|| DUMMY_HASH.clone());
    let password = payload.password;

    // Limiting RAM usage
    let _permit = state
        .hash_limit
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let password_ok = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !(user_exists && password_ok) {
        return Err(StatusCode::UNAUTHORIZED.into());
    }
    failures.refund(&username.to_string());

    let token =
        create_jwt(&state.jwt, username).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse { token }))
}

pub async fn create_login_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateLoginPayload>,
) -> Result<StatusCode, AuthError> {
    // Checked first, before the expensive admin code check
    let username = required_text(&payload.username, "username", 50)
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?
        .to_string();
    if !(8..=128).contains(&payload.password.chars().count()) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY.into());
    }

    // Counted for the whole server, like wrong passwords above, so spreading
    // guesses over many addresses doesn't help
    let failures = &state.login_limits.admin_code_failures;
    failures.check(()).map_err(AuthError::TooManyRequests)?;

    let hash = state.admin_hash.clone();
    let password = payload.admin_code;

    // Limiting RAM usage
    let _permit = state
        .hash_limit
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let password_ok = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !password_ok {
        return Err(StatusCode::UNAUTHORIZED.into());
    }
    failures.refund(&());

    let hashed_password = tokio::task::spawn_blocking(move || hash_password(&payload.password))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state
        .db
        .teachers
        .create(&username, &hashed_password)
        .await
        .map_err(|e| match e {
            AppDatabaseError::Conflict { .. } => StatusCode::CONFLICT,
            e => db_error(e),
        })?;

    Ok(StatusCode::CREATED)
}
