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

/// An `add_activity` message. The teacher comes from the connection's login token.
fn add_activity(student_id: i64, star_diff: i64, kind: &str, subject: &str) -> Value {
    json!({
        "type": "add_activity",
        "data": {
            "student_id": student_id,
            "reward_description": "Great homework",
            "star_diff": star_diff,
            "kind": kind,
            "subject": subject
        }
    })
}

/// The balances of a student with no stars yet
fn zero_balances() -> Value {
    json!({
        "art": 0, "ilp": 0, "maths": 0, "english": 0,
        "humanities": 0, "citizenship": 0, "vocational": 0, "options": 0
    })
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

#[tokio::test]
async fn websocket_refuses_a_token_for_a_teacher_that_does_not_exist() {
    let server = TestServer::start().await;

    // Correctly signed, but there is no teacher called "ghost"
    let token = create_jwt(&server.state.jwt, "ghost").unwrap();
    assert_eq!(server.ws_rejection_status(Some(&token)).await, 401);
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

    // The new student arrives in a broadcast, with the name trimmed and no stars in any class
    let broadcast = client.recv_json().await;
    assert_eq!(
        broadcast,
        json!({
            "id": null,
            "msg": {
                "type": "student_added",
                "data": {
                    "id": 1,
                    "name": "Alex Smith",
                    "ks": "ks3",
                    "balances": zero_balances(),
                    "earned_stars": 0
                }
            }
        })
    );

    let students = client.request(json!({ "type": "get_students" })).await;
    assert_eq!(students["data"][0], broadcast["msg"]["data"]);
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
async fn add_activity_updates_the_class_balance() {
    let server = TestServer::start().await;
    let (teacher_id, token) = server.teacher("teacher1").await;
    let student = server.state.db.students.create("Alex Smith", KeyStage::Ks3).await.unwrap();
    let mut client = server.connect(&token).await;

    let mut msg = add_activity(student.id, 3, "reward", "maths");
    msg["data"]["reward_description"] = json!("  Great homework  ");
    let reply = client.request(msg).await;
    assert_eq!(reply, json!({ "type": "ack" }));

    let broadcast = client.recv_json().await;
    assert_eq!(broadcast["id"], Value::Null);
    assert_eq!(broadcast["msg"]["type"], "activity_added");
    let data = &broadcast["msg"]["data"];
    assert_eq!(data["activity"]["teacher_id"], teacher_id);
    assert_eq!(data["activity"]["reward_description"], "Great homework");
    assert_eq!(data["activity"]["star_diff"], 3);
    assert_eq!(data["activity"]["kind"], "reward");
    assert_eq!(data["activity"]["subject"], "maths");
    assert_eq!(data["student"]["balances"]["maths"], 3);
    assert_eq!(data["student"]["balances"]["art"], 0);
    assert_eq!(data["student"]["earned_stars"], 3);

    // Cash some in: the Maths balance drops, earned doesn't
    let mut msg = add_activity(student.id, -1, "cash_in", "maths");
    msg["data"]["reward_description"] = json!("Sweets");
    client.request(msg).await;
    let broadcast = client.recv_json().await;
    assert_eq!(broadcast["msg"]["data"]["activity"]["kind"], "cash_in");
    assert_eq!(broadcast["msg"]["data"]["student"]["balances"]["maths"], 2);
    assert_eq!(broadcast["msg"]["data"]["student"]["earned_stars"], 3);

    // Newest first, with the kind and class stored
    let activities = client.request(json!({ "type": "get_activities" })).await;
    assert_eq!(activities["data"][0]["reward_description"], "Sweets");
    assert_eq!(activities["data"][0]["kind"], "cash_in");
    assert_eq!(activities["data"][0]["subject"], "maths");
    assert_eq!(activities["data"][1]["reward_description"], "Great homework");
    assert_eq!(activities["data"][1]["kind"], "reward");

    // get_students works the totals out the same way
    let students = client.request(json!({ "type": "get_students" })).await;
    assert_eq!(students["data"][0], broadcast["msg"]["data"]["student"]);
}

#[tokio::test]
async fn activities_are_recorded_under_the_logged_in_teacher() {
    let server = TestServer::start().await;
    let (alice_id, _) = server.teacher("alice").await;
    let (bob_id, bob_token) = server.teacher("bob").await;
    let student = server.state.db.students.create("Alex Smith", KeyStage::Ks3).await.unwrap();
    let mut bob = server.connect(&bob_token).await;

    // Bob tries to record stars as Alice. The extra field is ignored.
    let mut msg = add_activity(student.id, 2, "reward", "art");
    msg["data"]["teacher_id"] = json!(alice_id);
    assert_eq!(bob.request(msg).await, json!({ "type": "ack" }));

    let broadcast = bob.recv_json().await;
    assert_eq!(broadcast["msg"]["data"]["activity"]["teacher_id"], bob_id);
    let stored = server.state.db.activities.list().await.unwrap();
    assert_eq!(stored[0].teacher_id, bob_id);
}

#[tokio::test]
async fn add_activity_checks_kind_and_amount() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let student = server.state.db.students.create("Alex Smith", KeyStage::Ks3).await.unwrap();
    let mut client = server.connect(&token).await;

    // Give them 3 Maths stars to start with
    let reply = client.request(add_activity(student.id, 3, "reward", "maths")).await;
    assert_eq!(reply, json!({ "type": "ack" }));
    client.recv_json().await;

    let bad = [
        (-1, "reward", "maths"),      // rewards must be positive
        (1, "cash_in", "maths"),      // cash-ins must be negative
        (1, "take_away", "maths"),    // take-aways must be negative
        (0, "reward", "maths"),       // zero changes nothing
        (101, "reward", "maths"),     // too many at once
        (-101, "take_away", "maths"),
        (-5, "cash_in", "maths"),     // more than the 3 they have
        (-1, "cash_in", "art"),       // they have no Art stars
    ];
    for (star_diff, kind, subject) in bad {
        let reply = client.request(add_activity(student.id, star_diff, kind, subject)).await;
        assert_eq!(error_code(&reply), "invalid_input", "for {star_diff} {kind} {subject}");
    }

    // Nothing was written by the bad requests
    assert_eq!(server.state.db.activities.list().await.unwrap().len(), 1);
    let students = client.request(json!({ "type": "get_students" })).await;
    assert_eq!(students["data"][0]["balances"]["maths"], 3);
}

#[tokio::test]
async fn add_activity_for_unknown_student_is_not_found() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let mut client = server.connect(&token).await;

    let reply = client.request(add_activity(99, 1, "reward", "maths")).await;
    assert_eq!(error_code(&reply), "not_found");
    assert_eq!(reply["data"]["message"], "student 99 not found");
}

#[tokio::test]
async fn add_activity_checks_the_description() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let student = server.state.db.students.create("Alex Smith", KeyStage::Ks3).await.unwrap();
    let mut client = server.connect(&token).await;

    for description in ["   ".to_string(), "a".repeat(201)] {
        let mut msg = add_activity(student.id, 1, "reward", "maths");
        msg["data"]["reward_description"] = json!(description);
        let reply = client.request(msg).await;
        assert_eq!(error_code(&reply), "invalid_input");
    }

    assert!(server.state.db.activities.list().await.unwrap().is_empty());
}

// --- Removing ---

#[tokio::test]
async fn remove_activity_replies_ack_then_broadcasts() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let student = server.state.db.students.create("Alex Smith", KeyStage::Ks3).await.unwrap();
    let mut client = server.connect(&token).await;

    client.request(add_activity(student.id, 4, "reward", "english")).await;
    let added = client.recv_json().await;
    let activity_id = added["msg"]["data"]["activity"]["id"].clone();

    let reply = client
        .request(json!({ "type": "remove_activity", "data": { "activity_id": activity_id } }))
        .await;
    assert_eq!(reply, json!({ "type": "ack" }));

    let broadcast = client.recv_json().await;
    assert_eq!(broadcast["id"], Value::Null);
    assert_eq!(broadcast["msg"]["type"], "activity_removed");
    assert_eq!(broadcast["msg"]["data"]["activity_id"], activity_id);
    assert_eq!(broadcast["msg"]["data"]["student"]["balances"], zero_balances());
    assert_eq!(broadcast["msg"]["data"]["student"]["earned_stars"], 0);

    let activities = client.request(json!({ "type": "get_activities" })).await;
    assert_eq!(activities, json!({ "type": "activities", "data": [] }));

    // Removing it again
    let reply = client
        .request(json!({ "type": "remove_activity", "data": { "activity_id": activity_id } }))
        .await;
    assert_eq!(error_code(&reply), "not_found");
}

#[tokio::test]
async fn removing_a_spent_reward_is_invalid_input() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let student = server.state.db.students.create("Alex Smith", KeyStage::Ks3).await.unwrap();
    let mut client = server.connect(&token).await;

    client.request(add_activity(student.id, 5, "reward", "maths")).await;
    let reward_id = client.recv_json().await["msg"]["data"]["activity"]["id"].clone();
    client.request(add_activity(student.id, -5, "cash_in", "maths")).await;
    client.recv_json().await;

    let reply = client
        .request(json!({ "type": "remove_activity", "data": { "activity_id": reward_id } }))
        .await;
    assert_eq!(error_code(&reply), "invalid_input");
    assert_eq!(
        reply["data"]["message"],
        format!("student {} doesn't have enough Maths stars", student.id)
    );
    assert_eq!(server.state.db.activities.list().await.unwrap().len(), 2);
}

#[tokio::test]
async fn remove_student_replies_ack_then_broadcasts() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let student = server.state.db.students.create("Alex Smith", KeyStage::Ks3).await.unwrap();
    let mut client = server.connect(&token).await;

    client.request(add_activity(student.id, 2, "reward", "art")).await;
    client.recv_json().await;

    let reply = client
        .request(json!({ "type": "remove_student", "data": { "student_id": student.id } }))
        .await;
    assert_eq!(reply, json!({ "type": "ack" }));

    let broadcast = client.recv_json().await;
    assert_eq!(
        broadcast,
        json!({ "id": null, "msg": { "type": "student_removed", "data": { "student_id": student.id } } })
    );

    let students = client.request(json!({ "type": "get_students" })).await;
    assert_eq!(students, json!({ "type": "students", "data": [] }));
    let activities = client.request(json!({ "type": "get_activities" })).await;
    assert_eq!(activities, json!({ "type": "activities", "data": [] }));

    let reply = client
        .request(json!({ "type": "remove_student", "data": { "student_id": student.id } }))
        .await;
    assert_eq!(error_code(&reply), "not_found");
}

// --- Bad messages ---

#[tokio::test]
async fn unparseable_messages_get_bad_format_with_null_id() {
    let server = TestServer::start().await;
    let (_, token) = server.teacher("teacher1").await;
    let mut client = server.connect(&token).await;

    let unknown_type = json!({ "id": 1, "msg": { "type": "delete_everything" } }).to_string();
    let star_diff_too_big =
        json!({ "id": 2, "msg": add_activity(1, 40000, "reward", "maths") }).to_string();
    let unknown_class =
        json!({ "id": 3, "msg": add_activity(1, 1, "reward", "pottery") }).to_string();
    let mut no_class = add_activity(1, 1, "reward", "maths");
    no_class["data"].as_object_mut().unwrap().remove("subject");
    let no_class = json!({ "id": 4, "msg": no_class }).to_string();

    for text in [
        "not json",
        unknown_type.as_str(),
        star_diff_too_big.as_str(),
        unknown_class.as_str(),
        no_class.as_str(),
    ] {
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
