use std::sync::Arc;

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use tokio::sync::broadcast::error::RecvError;

use crate::{
    AppState,
    database::{CreateActivity, KeyStage},
    messaging::{
        error::{AppResult, ServerError},
        protocol::{ClientMsg, ClientMsgType, ServerMsg, ServerMsgType},
    },
};

async fn send_json(socket: &mut WebSocket, msg: &ServerMsg) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).expect("ServerMsg should always serialize");
    socket.send(Message::Text(text.into())).await
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Limiting message and frame size
    ws.max_message_size(16 * 1024)
        .max_frame_size(16 * 1024)
        .on_failed_upgrade(|error| tracing::warn!("Error upgrading websocket: {error}"))
        .on_upgrade(move |socket| handle_socket(socket, state))
}

pub fn send_broadcast(msg: ServerMsgType, state: &AppState) {
    let _ = state.tx.send(ServerMsg { id: None, msg });
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();

    loop {
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(msg)) = incoming else { break };

                let text = match msg {
                    Message::Text(text) => text,
                    Message::Close(_) => break,
                    _ => continue,
                };

                // Every incoming message gets exactly one reply.
                let reply = match serde_json::from_str::<ClientMsg>(&text) {
                    Ok(ClientMsg { id, msg }) => ServerMsg {
                        id: Some(id),
                        msg: process(&state, msg).await,
                    },
                    Err(err) => ServerMsg {
                        id: None,
                        msg: ServerMsgType::Error {
                            code: "bad_format",
                            message: err.to_string(),
                        },
                    },
                };

                if send_json(&mut socket, &reply).await.is_err() {
                    break;
                }
            }

            broadcast = rx.recv() => {
                match broadcast {
                    Ok(msg) => {
                        if send_json(&mut socket, &msg).await.is_err() {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }
}

async fn process(state: &AppState, msg: ClientMsgType) -> ServerMsgType {
    let result = match msg {
        // Gets
        ClientMsgType::GetStudents => handle_get_students(state).await,
        ClientMsgType::GetTeachers => handle_get_teachers(state).await,
        ClientMsgType::GetActivities => handle_get_activities(state).await,

        // Changes
        ClientMsgType::AddStudent {
            student_name,
            key_stage,
        } => handle_add_student(state, student_name, key_stage).await,
        ClientMsgType::AddActivity {
            student_id,
            teacher_id,
            reward_description,
            star_diff,
        } => {
            handle_add_activity(state, student_id, teacher_id, reward_description, star_diff).await
        }
    };

    result.unwrap_or_else(error_msg)
}

fn error_msg(err: ServerError) -> ServerMsgType {
    if err.is_internal() {
        tracing::error!(?err, "request failed");
    }
    ServerMsgType::Error {
        code: err.code(),
        message: err.client_message(),
    }
}

async fn handle_get_students(state: &AppState) -> AppResult<ServerMsgType> {
    let students = state.db.students.list().await?;
    Ok(ServerMsgType::Students(students))
}

async fn handle_get_teachers(state: &AppState) -> AppResult<ServerMsgType> {
    let teachers = state.db.teachers.list().await?;
    Ok(ServerMsgType::Teachers(teachers))
}

async fn handle_get_activities(state: &AppState) -> AppResult<ServerMsgType> {
    let activities = state.db.activities.list().await?;
    Ok(ServerMsgType::Activities(activities))
}

async fn handle_add_student(
    state: &AppState,
    student_name: String,
    key_stage: String,
) -> AppResult<ServerMsgType> {
    let student_name = student_name.trim();
    if student_name.is_empty() {
        return Err(ServerError::Validation(
            "student name must not be empty".to_string(),
        ));
    }

    let key_stage = match key_stage.trim().to_lowercase().as_str() {
        "ks3" => KeyStage::Ks3,
        "ks4" => KeyStage::Ks4,
        _ => {
            return Err(ServerError::Validation(
                "key_stage must be \"ks3\" or \"ks4\"".to_string(),
            ));
        }
    };

    let student = state.db.students.create(student_name, key_stage).await?;

    send_broadcast(ServerMsgType::StudentAdded(student.clone()), state);
    Ok(ServerMsgType::Ack)
}

async fn handle_add_activity(
    state: &AppState,
    student_id: i64,
    teacher_id: i64,
    reward_description: String,
    star_diff: i16,
) -> AppResult<ServerMsgType> {
    let (activity, student) = state
        .db
        .activities
        .create(CreateActivity {
            student_id,
            teacher_id,
            reward_description,
            star_diff,
        })
        .await?;

    send_broadcast(
        ServerMsgType::ActivityAdded {
            activity: activity.clone(),
            student: student.clone(),
        },
        state,
    );
    Ok(ServerMsgType::Ack)
}
