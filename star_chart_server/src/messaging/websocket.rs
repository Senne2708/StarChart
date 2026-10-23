use std::sync::Arc;

use axum::{extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}}, response::IntoResponse};
use tokio::sync::broadcast::error::RecvError;

use crate::{AppState, messaging::protocol::{ClientMsg, ClientMsgType, ServerMsg, ServerMsgType}};

enum Outcome {
    Reply(ServerMsgType),
    Broadcast(ServerMsgType),
}


async fn send_json(socket: &mut WebSocket, msg: &ServerMsg) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).expect("ServerMsg should always serialize");
    socket.send(Message::Text(text.into())).await
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_failed_upgrade(|error| println!("Error upgrading websocket: {error}"))
        .on_upgrade(move |socket| handle_socket(socket, state))
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

                let reply = match serde_json::from_str::<ClientMsg>(&text) {
                    Ok(ClientMsg { id, msg }) => match process(&state, msg) {
                        Outcome::Reply(msg) => Some(ServerMsg { id: Some(id), msg }),
                        Outcome::Broadcast(msg) => {
                            let _ = state.tx.send(ServerMsg { id: None, msg });
                            None
                        }
                    },
                    Err(err) => Some(ServerMsg {
                        id: None,
                        msg: ServerMsgType::Error {
                            code: "bad_format",
                            message: err.to_string(),
                        },
                    }),
                };

                if let Some(reply) = reply {
                    if send_json(&mut socket, &reply).await.is_err() {
                        break;
                    }
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


fn process(state: &AppState, msg: ClientMsgType) -> Outcome {
    let mut store = state.store.lock().unwrap();

    match msg {
        // Gets
        ClientMsgType::GetStudents => Outcome::Reply(ServerMsgType::Students(
            store.students.values().cloned().collect(),
        )),
        ClientMsgType::GetTeachers => Outcome::Reply(ServerMsgType::Teachers(
            store.teachers.values().cloned().collect(),
        )),
        ClientMsgType::GetActivities => {
            Outcome::Reply(ServerMsgType::Activities(store.activities.clone()))
        }

        // Changes
        ClientMsgType::AddStudent { student_name } => {
            Outcome::Broadcast(ServerMsgType::StudentAdded(store.add_student(student_name)))
        }
        ClientMsgType::AddTeacher { teacher_name } => {
            Outcome::Broadcast(ServerMsgType::TeacherAdded(store.add_teacher(teacher_name)))
        }
        ClientMsgType::AddActivity {
            student_id,
            teacher_id,
            star_diff,
        } => {
            match store.add_activity(student_id, teacher_id, star_diff) {
                Ok((activity, student)) => {
                    Outcome::Broadcast(ServerMsgType::ActivityAdded { activity, student })
                }
                Err(code) => Outcome::Reply(ServerMsgType::Error {
                    code,
                    message: "Could not add activity".to_string(),
                }),
            }
        }
    }
}

