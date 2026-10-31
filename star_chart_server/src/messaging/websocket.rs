use std::sync::Arc;

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    }, response::IntoResponse,
};
use tokio::sync::broadcast::error::RecvError;

use crate::{
    AppState, auth::password::hash_password, database::{AppDatabaseError, CreateActivity}, messaging::protocol::{ClientMsg, ClientMsgType, ServerMsg, ServerMsgType},
};

async fn send_json(socket: &mut WebSocket, msg: &ServerMsg) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).expect("ServerMsg should always serialize");
    socket.send(Message::Text(text.into())).await
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_failed_upgrade(|error| tracing::warn!("Error upgrading websocket: {error}"))
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
        ClientMsgType::AddStudent { student_name } => {
            handle_add_student(state, student_name).await
        }
        ClientMsgType::AddActivity {
            student_id,
            teacher_id,
            reward_description,
            star_diff,
        } => handle_add_activity(state, student_id, teacher_id, reward_description, star_diff).await,
    };

    result.unwrap_or_else(error_msg)
}

fn error_msg(err: AppDatabaseError) -> ServerMsgType {
    let code = match &err {
        AppDatabaseError::NotFound { .. } => "not_found",
        AppDatabaseError::Internal(e) => {
            tracing::error!(?e, "database error");
            "internal_error"
        }
        AppDatabaseError::Temp{ .. } => "temp"
    };
    ServerMsgType::Error {
        code,
        message: err.client_message(),
    }
}

async fn handle_get_students(state: &AppState) -> Result<ServerMsgType, AppDatabaseError> {
    let students = state.db.students.list().await?;
    Ok(ServerMsgType::Students(students))
}

async fn handle_get_teachers(state: &AppState) -> Result<ServerMsgType, AppDatabaseError> {
    let teachers = state.db.teachers.list().await?;
    Ok(ServerMsgType::Teachers(teachers))
}

async fn handle_get_activities(state: &AppState) -> Result<ServerMsgType, AppDatabaseError> {
    let activities = state.db.activities.list().await?;
    Ok(ServerMsgType::Activities(activities))
}

async fn handle_add_student(
    state: &AppState,
    student_name: String,
) -> Result<ServerMsgType, AppDatabaseError> {
    let student = state.db.students.create(&student_name).await?;

    send_broadcast(ServerMsgType::StudentAdded(student.clone()), state);
    Ok(ServerMsgType::StudentAdded(student))
}

async fn handle_add_activity(
    state: &AppState,
    student_id: i64,
    teacher_id: i64,
    reward_description: String,
    star_diff: i16,
) -> Result<ServerMsgType, AppDatabaseError> {
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
    Ok(ServerMsgType::ActivityAdded { activity, student })
}
