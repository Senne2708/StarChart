use std::sync::Arc;

use axum::{
    Extension,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::Response,
};
use tokio::sync::broadcast::error::RecvError;

use crate::{
    AppState,
    auth::jwt_token::Claims,
    database::{CreateActivity, db_error},
    messaging::{
        error::{AppResult, ServerError},
        protocol::{
            ActivityKind, ClientMsg, ClientMsgType, ErrorCode, KeyStage, ServerMsg, ServerMsgType,
        },
        validation::{required_text, validate_star_change},
    },
};

async fn send_json(socket: &mut WebSocket, msg: &ServerMsg) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).expect("ServerMsg should always serialize");
    socket.send(Message::Text(text.into())).await
}

/// Who is on the other end of a WebSocket. Set once, from the login token, when it opens.
struct Session {
    teacher_id: i64,
}

/// Opens a WebSocket for the teacher named in the login token. `auth_middleware` has
/// already checked the token. Refuses with `401` if that teacher no longer exists.
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Response, StatusCode> {
    let teacher = state
        .db
        .teachers
        .get_by_name(&claims.sub)
        .await
        .map_err(db_error)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let session = Session { teacher_id: teacher.id };

    // Limiting message and frame size
    Ok(ws
        .max_message_size(16 * 1024)
        .max_frame_size(16 * 1024)
        .on_failed_upgrade(|error| tracing::warn!("Error upgrading websocket: {error}"))
        .on_upgrade(move |socket| handle_socket(socket, state, session)))
}

pub fn send_broadcast(msg: ServerMsgType, state: &AppState) {
    let _ = state.tx.send(ServerMsg { id: None, msg });
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, session: Session) {
    let mut rx = state.tx.subscribe();
    let mut shutdown = state.shutdown.clone();

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
                        msg: process(&state, &session, msg).await,
                    },
                    Err(err) => ServerMsg {
                        id: None,
                        msg: ServerMsgType::Error {
                            code: ErrorCode::BadFormat,
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
                    // Missed broadcasts, tell the client to re-fetch
                    Err(RecvError::Lagged(_)) => {
                        let resync = ServerMsg { id: None, msg: ServerMsgType::Resync };
                        if send_json(&mut socket, &resync).await.is_err() {
                            break;
                        }
                    }
                    Err(RecvError::Closed) => break,
                }
            }

            // Close open sockets so graceful shutdown doesn't wait on them
            _ = shutdown.changed() => {
                let _ = socket.send(Message::Close(None)).await;
                break;
            }
        }
    }
}

async fn process(state: &AppState, session: &Session, msg: ClientMsgType) -> ServerMsgType {
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
            reward_description,
            star_diff,
            kind,
            subject,
        } => {
            let input = CreateActivity {
                student_id,
                // Always the logged-in teacher, never something the client chose
                teacher_id: session.teacher_id,
                reward_description,
                star_diff,
                kind,
                subject,
            };
            handle_add_activity(state, input).await
        }

        // Removals
        ClientMsgType::RemoveActivity { activity_id } => {
            handle_remove_activity(state, activity_id).await
        }
        ClientMsgType::RemoveStudent { student_id } => {
            handle_remove_student(state, student_id).await
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
    key_stage: KeyStage,
) -> AppResult<ServerMsgType> {
    let student_name = required_text(&student_name, "student name", 100)?;

    let student = state.db.students.create(student_name, key_stage).await?;

    send_broadcast(ServerMsgType::StudentAdded(student), state);
    Ok(ServerMsgType::Ack)
}

async fn handle_add_activity(
    state: &AppState,
    mut input: CreateActivity,
) -> AppResult<ServerMsgType> {
    validate_star_change(input.kind, input.star_diff)?;
    let field = match input.kind {
        ActivityKind::CashIn => "reward",
        ActivityKind::Reward | ActivityKind::TakeAway => "reason",
    };
    input.reward_description = required_text(&input.reward_description, field, 200)?.to_string();

    let (activity, student) = state.db.activities.create(input).await?;

    send_broadcast(ServerMsgType::ActivityAdded { activity, student }, state);
    Ok(ServerMsgType::Ack)
}

async fn handle_remove_activity(state: &AppState, activity_id: String) -> AppResult<ServerMsgType> {
    let student = state.db.activities.remove(&activity_id).await?;

    send_broadcast(ServerMsgType::ActivityRemoved { activity_id, student }, state);
    Ok(ServerMsgType::Ack)
}

async fn handle_remove_student(state: &AppState, student_id: i64) -> AppResult<ServerMsgType> {
    state.db.students.remove(student_id).await?;

    send_broadcast(ServerMsgType::StudentRemoved { student_id }, state);
    Ok(ServerMsgType::Ack)
}
