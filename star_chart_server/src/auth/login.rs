use std::sync::{Arc, LazyLock};

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::{
    AppState, auth::{
        jwt_token::create_jwt, password::{self, hash_password, verify_password},
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

static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| hash_password("dummy-password").expect("failed to hash dummy password"));

static ADMIN_HASH: LazyLock<String> = LazyLock::new(|| hash_password("admin-password").expect("failed to hash dummy password"));

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let stored_hash = {
        let store = state.store.lock().unwrap();
        store.get_user(&payload.username).cloned()
    };

    let user_exists = stored_hash.is_some();
    let hash = stored_hash.unwrap_or_else(|| DUMMY_HASH.clone());
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

    let hash = ADMIN_HASH.clone();
    let password = payload.admin_code;


    let password_ok = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;


    if password_ok {
        let mut store = state.store.lock().unwrap();
        store.add_user(&payload.username, &payload.password);
        return Ok(StatusCode::OK);
    }

    Ok(StatusCode::UNAUTHORIZED)

}
