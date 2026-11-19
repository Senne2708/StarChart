use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Serialize)]
struct LoginPayload<'a> {
    username: &'a str,
    password: &'a str,
}

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}

#[derive(Serialize)]
struct CreateLoginPayload<'a> {
    username: &'a str,
    password: &'a str,
    admin_code: &'a str,
}

/// Swaps a username and password for a token, with `POST /login`
pub async fn login(
    http: &reqwest::Client,
    server_url: &str,
    username: &str,
    password: &str,
) -> AppResult<String> {
    let response = http
        .post(format!("{server_url}/login"))
        .json(&LoginPayload { username, password })
        .send()
        .await?;

    if response.status() == StatusCode::UNAUTHORIZED {
        return Err(AppError::BadLogin);
    }

    let body: LoginResponse = response.error_for_status()?.json().await?;
    Ok(body.token)
}

/// Creates a teacher account with `POST /create_login`. Needs the server's admin code.
pub async fn create_login(
    http: &reqwest::Client,
    server_url: &str,
    username: &str,
    password: &str,
    admin_code: &str,
) -> AppResult<()> {
    let response = http
        .post(format!("{server_url}/create_login"))
        .json(&CreateLoginPayload {
            username,
            password,
            admin_code,
        })
        .send()
        .await?;

    match response.status() {
        StatusCode::UNAUTHORIZED => Err(AppError::BadAdminCode),
        StatusCode::CONFLICT => Err(AppError::UsernameTaken),
        StatusCode::UNPROCESSABLE_ENTITY => Err(AppError::InvalidAccount),
        _ => {
            response.error_for_status()?;
            Ok(())
        }
    }
}
