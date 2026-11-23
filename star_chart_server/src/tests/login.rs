use std::time::Duration;

use serde_json::{Value, json};

use super::support::{ADMIN_CODE, TestServer, generous_limits};
use crate::auth::rate_limit::{LoginLimits, RateLimiter};

async fn create_login(server: &TestServer, username: &str, password: &str, admin_code: &str) -> u16 {
    let body = json!({ "username": username, "password": password, "admin_code": admin_code });
    server.post_json("/create_login", body).await.status().as_u16()
}

async fn login(server: &TestServer, username: &str, password: &str) -> reqwest::Response {
    let body = json!({ "username": username, "password": password });
    server.post_json("/login", body).await
}

#[tokio::test]
async fn created_teacher_can_log_in_and_connect() {
    let server = TestServer::start().await;
    assert_eq!(create_login(&server, "teacher1", "password123", ADMIN_CODE).await, 201);

    let response = login(&server, "teacher1", "password123").await;
    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.unwrap();
    let token = body["token"].as_str().expect("token should be a string");

    // The token opens a WebSocket
    server.connect(token).await;
}

#[tokio::test]
async fn create_login_needs_the_admin_code() {
    let server = TestServer::start().await;
    assert_eq!(create_login(&server, "teacher1", "password123", "wrong").await, 401);
    assert!(server.state.db.teachers.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn create_login_rejects_a_taken_username() {
    let server = TestServer::start().await;
    assert_eq!(create_login(&server, "teacher1", "password123", ADMIN_CODE).await, 201);
    assert_eq!(create_login(&server, "teacher1", "other-password", ADMIN_CODE).await, 409);
}

#[tokio::test]
async fn create_login_checks_username_and_password() {
    let server = TestServer::start().await;
    let long_name = "a".repeat(51);
    let long_password = "a".repeat(129);

    for (username, password) in [
        ("", "password123"),
        ("   ", "password123"),
        (long_name.as_str(), "password123"),
        ("teacher1", "short"),
        ("teacher1", long_password.as_str()),
    ] {
        assert_eq!(
            create_login(&server, username, password, ADMIN_CODE).await,
            422,
            "username {username:?}, password {password:?}"
        );
    }
    assert!(server.state.db.teachers.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn usernames_are_trimmed() {
    let server = TestServer::start().await;
    assert_eq!(create_login(&server, "  teacher1  ", "password123", ADMIN_CODE).await, 201);

    let teachers = server.state.db.teachers.list().await.unwrap();
    assert_eq!(teachers[0].name, "teacher1");
    assert_eq!(login(&server, " teacher1 ", "password123").await.status(), 200);
}

#[tokio::test]
async fn login_rejects_wrong_password_and_unknown_user() {
    let server = TestServer::start().await;
    assert_eq!(create_login(&server, "teacher1", "password123", ADMIN_CODE).await, 201);

    assert_eq!(login(&server, "teacher1", "wrong-password").await.status(), 401);
    assert_eq!(login(&server, "nobody", "password123").await.status(), 401);
}

// --- Rate limits ---

/// 2 attempts, one back every 30 seconds
fn two_per_minute<K: Eq + std::hash::Hash>() -> RateLimiter<K> {
    RateLimiter::new(2, Duration::from_secs(60))
}

fn retry_after(response: &reqwest::Response) -> u64 {
    response.headers()["retry-after"].to_str().unwrap().parse().unwrap()
}

#[tokio::test]
async fn requests_are_limited_per_address() {
    let limits = LoginLimits {
        per_ip: two_per_minute(),
        ..generous_limits()
    };
    let server = TestServer::start_with_limits(limits).await;

    assert_eq!(login(&server, "nobody", "password123").await.status(), 401);
    assert_eq!(login(&server, "nobody", "password123").await.status(), 401);

    let response = login(&server, "nobody", "password123").await;
    assert_eq!(response.status(), 429);
    let wait = retry_after(&response);
    assert!((1..=30).contains(&wait), "retry-after was {wait}");

    // `/create_login` shares the same limit
    assert_eq!(create_login(&server, "teacher1", "password123", ADMIN_CODE).await, 429);
}

#[tokio::test]
async fn forwarded_for_separates_clients_behind_a_proxy() {
    let limits = LoginLimits {
        per_ip: RateLimiter::new(1, Duration::from_secs(60)),
        ..generous_limits()
    };
    let server = TestServer::start_with_limits(limits).await;
    let body = json!({ "username": "nobody", "password": "password123" });

    // The test connects from 127.0.0.1, so the server trusts the header like it would from a proxy
    let first = server.post_json_forwarded_for("/login", body.clone(), "203.0.113.1").await;
    assert_eq!(first.status(), 401);
    let again = server.post_json_forwarded_for("/login", body.clone(), "203.0.113.1").await;
    assert_eq!(again.status(), 429);

    let other_client = server.post_json_forwarded_for("/login", body, "203.0.113.2").await;
    assert_eq!(other_client.status(), 401);
}

#[tokio::test]
async fn wrong_passwords_lock_only_that_username() {
    let limits = LoginLimits {
        password_failures: two_per_minute(),
        ..generous_limits()
    };
    let server = TestServer::start_with_limits(limits).await;
    assert_eq!(create_login(&server, "teacher1", "password123", ADMIN_CODE).await, 201);

    assert_eq!(login(&server, "teacher1", "wrong-password").await.status(), 401);
    assert_eq!(login(&server, "teacher1", "wrong-password").await.status(), 401);

    // Locked, even with the right password, so guessing can't continue
    let response = login(&server, "teacher1", "password123").await;
    assert_eq!(response.status(), 429);
    assert!(retry_after(&response) > 0);

    // Other usernames aren't affected, so one teacher's typos don't lock out the school
    assert_eq!(login(&server, "teacher2", "password123").await.status(), 401);
}

#[tokio::test]
async fn right_passwords_do_not_count_as_failures() {
    let limits = LoginLimits {
        password_failures: RateLimiter::new(1, Duration::from_secs(60)),
        ..generous_limits()
    };
    let server = TestServer::start_with_limits(limits).await;
    assert_eq!(create_login(&server, "teacher1", "password123", ADMIN_CODE).await, 201);

    for _ in 0..3 {
        assert_eq!(login(&server, "teacher1", "password123").await.status(), 200);
    }
}

#[tokio::test]
async fn wrong_admin_codes_are_limited() {
    let limits = LoginLimits {
        admin_code_failures: RateLimiter::new(1, Duration::from_secs(60)),
        ..generous_limits()
    };
    let server = TestServer::start_with_limits(limits).await;

    assert_eq!(create_login(&server, "teacher1", "password123", "wrong").await, 401);

    // Locked, even with the right code
    assert_eq!(create_login(&server, "teacher1", "password123", ADMIN_CODE).await, 429);
}

#[tokio::test]
async fn bad_request_bodies_are_rejected() {
    let server = TestServer::start().await;

    // No `Content-Type: application/json` header
    let response = server.post_raw("/login", None, "{}").await;
    assert_eq!(response.status(), 415);

    // Not valid JSON
    let response = server
        .post_raw("/login", Some("application/json"), "{ not json")
        .await;
    assert_eq!(response.status(), 400);

    // Valid JSON, but missing the password
    let response = server.post_json("/login", json!({ "username": "teacher1" })).await;
    assert_eq!(response.status(), 422);

    // Larger than the 4 KiB limit
    let response = login(&server, "teacher1", &"a".repeat(5000)).await;
    assert_eq!(response.status(), 413);
}
