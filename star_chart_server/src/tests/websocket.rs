use serde_json::{Value, json};
use star_chart_protocol::KeyStage;
use tokio_tungstenite::tungstenite::Message;

use super::support::TestServer;
use crate::{
    auth::jwt_token::{JwtKeys, create_jwt},
    messaging::protocol::{ServerMsg, ServerMsgType},
};

fn error_code(msg: &Value) -> &Value {
    assert_eq!(msg["type"], "error", "expected an error, got {msg}");
    &msg["data"]["code"]
}

// --- Connecting ---

#[tokio::test]
async fn websocket_needs_a_valid_token() {
    let server = TestServer::start().await;
    server.teacher("teacher1").await;

    assert_eq!(server.ws_rejection_status(None).await, 401);
    assert_eq!(server.ws_rejection_status(Some("not-a-token")).await, 401);

    let other_keys = JwtKeys::from_secret(b"another-secret-that-is-32-chars-long");
    let forged = create_jwt(&other_keys, "teacher1").unwrap();
    assert_eq!(server.ws_rejection_status(Some(&forged)).await, 401);
}

// --- Reading data ---

#[tokio::test]
async fn empty_database_returns_empty_lists() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let mut client = server.connect(&token).await;

    let students = client.request(json!({ "type": "get_students" })).await;
    assert_eq!(students, json!({ "type": "students", "data": [] }));

    let activities = client.request(json!({ "type": "get_activities" })).await;
    assert_eq!(activities, json!({ "type": "activities", "data": [] }));
}

#[tokio::test]
async fn teachers_never_include_the_password_hash() {
    let server = TestServer::start().await;
    let (teacher_id, token) = server.teacher("teacher1").await;
    let mut client = server.connect(&token).await;

    let teachers = client.request(json!({ "type": "get_teachers" })).await;
    assert_eq!(
        teachers,
        json!({ "type": "teachers", "data": [{ "id": teacher_id, "name": "teacher1" }] })
    );
}

// --- Adding students ---

#[tokio::test]
async fn add_student_replies_ack_then_broadcasts() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let mut client = server.connect(&token).await;

    let reply = client
        .request(json!({
            "type": "add_student",
            "data": { "student_name": "  Alex Smith  ", "key_stage": "ks3" }
        }))
        .await;
    assert_eq!(reply, json!({ "type": "ack" }));

    // The new student arrives in a broadcast, with the name trimmed
    let broadcast = client.recv_json().await;
    assert_eq!(
        broadcast,
        json!({
            "id": null,
            "msg": {
                "type": "student_added",
                "data": { "id": 1, "name": "Alex Smith", "ks": "ks3", "total_stars": 0 }
            }
        })
    );

    let students = client.request(json!({ "type": "get_students" })).await;
    assert_eq!(students["data"][0]["name"], "Alex Smith");
}

#[tokio::test]
async fn broadcasts_reach_every_client() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let mut sender = server.connect(&token).await;
    let mut other = server.connect(&token).await;

    sender
        .request(json!({
            "type": "add_student",
            "data": { "student_name": "Alex Smith", "key_stage": "ks4" }
        }))
        .await;

    // The other client didn't ask for anything, but still hears about the change
    let broadcast = other.recv_json().await;
    assert_eq!(broadcast["msg"]["type"], "student_added");
    assert_eq!(broadcast["msg"]["data"]["ks"], "ks4");
}

#[tokio::test]
async fn add_student_checks_the_name() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let mut client = server.connect(&token).await;

    for name in ["".to_string(), "   ".to_string(), "a".repeat(101)] {
        let reply = client
            .request(json!({
                "type": "add_student",
                "data": { "student_name": name, "key_stage": "ks3" }
            }))
            .await;
        assert_eq!(error_code(&reply), "invalid_input");
    }

    assert!(server.state.db.students.list().await.unwrap().is_empty());
}

// --- Adding activities ---

#[tokio::test]
async fn add_activity_updates_the_star_total() {
    let server = TestServer::start().await;
    let (teacher_id, token) = server.teacher("teacher1").await;
    let student = server.state.db.students.create("Alex Smith", KeyStage::Ks3).await.unwrap();
    let mut client = server.connect(&token).await;

    let reply = client
        .request(json!({
            "type": "add_activity",
            "data": {
                "student_id": student.id,
                "teacher_id": teacher_id,
                "reward_description": "  Great homework  ",
                "star_diff": 3
            }
        }))
        .await;
    assert_eq!(reply, json!({ "type": "ack" }));

    let broadcast = client.recv_json().await;
    assert_eq!(broadcast["id"], Value::Null);
    assert_eq!(broadcast["msg"]["type"], "activity_added");
    let data = &broadcast["msg"]["data"];
    assert_eq!(data["activity"]["reward_description"], "Great homework");
    assert_eq!(data["activity"]["star_diff"], 3);
    assert_eq!(data["student"]["total_stars"], 3);

    // Cash some in
    client
        .request(json!({
            "type": "add_activity",
            "data": {
                "student_id": student.id,
                "teacher_id": teacher_id,
                "reward_description": "Sweets",
                "star_diff": -1
            }
        }))
        .await;
    let broadcast = client.recv_json().await;
    assert_eq!(broadcast["msg"]["data"]["student"]["total_stars"], 2);

    // Newest first
    let activities = client.request(json!({ "type": "get_activities" })).await;
    assert_eq!(activities["data"][0]["reward_description"], "Sweets");
    assert_eq!(activities["data"][1]["reward_description"], "Great homework");
}

#[tokio::test]
async fn add_activity_for_unknown_student_is_not_found() {
    let server = TestServer::start().await;
    let (teacher_id, token) = server.teacher("teacher1").await;
    let mut client = server.connect(&token).await;

    let reply = client
        .request(json!({
            "type": "add_activity",
            "data": {
                "student_id": 99,
                "teacher_id": teacher_id,
                "reward_description": "Great homework",
                "star_diff": 1
            }
        }))
        .await;
    assert_eq!(error_code(&reply), "not_found");
    assert_eq!(reply["data"]["message"], "student 99 not found");
}

#[tokio::test]
async fn add_activity_checks_the_description() {
    let server = TestServer::start().await;
    let (teacher_id, token) = server.teacher("teacher1").await;
    let student = server.state.db.students.create("Alex Smith", KeyStage::Ks3).await.unwrap();
    let mut client = server.connect(&token).await;

    for description in ["   ".to_string(), "a".repeat(201)] {
        let reply = client
            .request(json!({
                "type": "add_activity",
                "data": {
                    "student_id": student.id,
                    "teacher_id": teacher_id,
                    "reward_description": description,
                    "star_diff": 1
                }
            }))
            .await;
        assert_eq!(error_code(&reply), "invalid_input");
    }

    assert!(server.state.db.activities.list().await.unwrap().is_empty());
}

// --- Bad messages ---

#[tokio::test]
async fn unparseable_messages_get_bad_format_with_null_id() {
    let server = TestServer::start().await;
    let (teacher_id, token) = server.teacher("teacher1").await;
    let mut client = server.connect(&token).await;

    let unknown_type = json!({ "id": 1, "msg": { "type": "delete_everything" } }).to_string();
    let star_diff_too_big = json!({
        "id": 2,
        "msg": {
            "type": "add_activity",
            "data": {
                "student_id": 1,
                "teacher_id": teacher_id,
                "reward_description": "Too many",
                "star_diff": 40000
            }
        }
    })
    .to_string();

    for text in ["not json", unknown_type.as_str(), star_diff_too_big.as_str()] {
        client.send_text(text).await;
        let reply = client.recv_json().await;
        assert_eq!(reply["id"], Value::Null, "for {text}");
        assert_eq!(error_code(&reply["msg"]), "bad_format", "for {text}");
    }

    // The connection still works afterwards
    client.request(json!({ "type": "get_students" })).await;
}

// --- Resync and shutdown ---

#[tokio::test]
async fn client_that_falls_behind_gets_resync() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let mut client = server.connect(&token).await;

    // Tests run on one thread, so the server can't forward anything until this
    // test awaits. Overflow the 64 message broadcast buffer before that happens.
    for _ in 0..100 {
        let _ = server.state.tx.send(ServerMsg {
            id: None,
            msg: ServerMsgType::Ack,
        });
    }

    let first = client.recv_json().await;
    assert_eq!(first, json!({ "id": null, "msg": { "type": "resync" } }));

    // After the resync, the newest broadcasts still arrive
    let next = client.recv_json().await;
    assert_eq!(next["msg"]["type"], "ack");
}

#[tokio::test]
async fn shutdown_closes_open_websockets() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let mut client = server.connect(&token).await;

    server.shutdown_tx.send(()).unwrap();

    let frame = client.recv().await;
    assert!(matches!(frame, Message::Close(_)), "expected a close frame, got {frame:?}");
}
