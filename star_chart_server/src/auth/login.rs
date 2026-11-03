use std::sync::{Arc, LazyLock};

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::{
    AppState, auth::{
        jwt_token::create_jwt, password::{hash_password, verify_password},
    }, database::db_error, messaging::error::{AppResult, ServerError},
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

static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| hash_password("dummy-password").expect("failed to hash dummy password"));

pub async fn get_admin_hash() -> AppResult<String> {
    let admin_password = match std::env::var("ADMIN_PASSWORD") {
        Ok(password) => password,
        
        // Only compiled into debug builds
        #[cfg(debug_assertions)]
        Err(_) => {
            tracing::warn!("ADMIN_PASSWORD not set, using insecure debug key");
            "debug_password".to_string()
        },

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
) -> Result<Json<LoginResponse>, StatusCode> {

    let user = state.db.teachers
        .get_password_hash(&payload.username)
        .await
        .map_err(db_error)?;

    let user_exists = user.is_some();
    let hash = user.unwrap_or_else(|| DUMMY_HASH.clone());
    let password = payload.password;

    let password_ok = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !(user_exists && password_ok) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token =
        create_jwt(&state.jwt, &payload.username).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse { token }))
}

pub async fn create_login_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateLoginPayload>,
) -> Result<StatusCode, StatusCode> {

    let hash = state.admin_hash.clone();
    let password = payload.admin_code;


    let password_ok = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if password_ok {
        let hashed_password = tokio::task::spawn_blocking(move || hash_password(&payload.password))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let _ = state.db.teachers.create(&payload.username, &hashed_password).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);

        return Ok(StatusCode::OK);
    }

    

    Ok(StatusCode::UNAUTHORIZED)
}
