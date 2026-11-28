//! The functions the React UI can call with `invoke`.
//!
//! Each one is thin: it turns the call into a protocol message, sends it, and
//! checks the reply is the expected type. Tauri renames the arguments to
//! camelCase for JavaScript, so `student_name` is passed as `studentName`.

use star_chart_protocol::{
    Activity, ActivityKind, ClientMsgType, KeyStage, ServerMsgType, Student, Subject, Teacher,
};
use tauri::{AppHandle, State};

use crate::{
    auth,
    error::{AppError, AppResult},
    server::ServerConnection,
    state::AppState,
};

/// Changes reply with `ack`. The new data arrives separately, as a `server-event`.
fn expect_ack(reply: ServerMsgType) -> AppResult<()> {
    match reply {
        ServerMsgType::Ack => Ok(()),
        _ => Err(AppError::UnexpectedReply),
    }
}

// --- Session ---

#[tauri::command]
pub async fn create_login(
    state: State<'_, AppState>,
    username: String,
    password: String,
    admin_code: String,
) -> AppResult<()> {
    state.ensure_secure_server()?;
    auth::create_login(&state.http, &state.server_url, &username, &password, &admin_code).await
}

#[tauri::command]
pub async fn login(
    app: AppHandle,
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> AppResult<()> {
    state.ensure_secure_server()?;
    let token = auth::login(&state.http, &state.server_url, &username, &password).await?;
    let connection = ServerConnection::connect(&state.ws_url(), &token, app).await?;
    state.set_connection(Some(connection)).await;
    Ok(())
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> AppResult<()> {
    state.set_connection(None).await;
    Ok(())
}

// --- Reading data ---

#[tauri::command]
pub async fn get_students(state: State<'_, AppState>) -> AppResult<Vec<Student>> {
    match state.request(ClientMsgType::GetStudents).await? {
        ServerMsgType::Students(students) => Ok(students),
        _ => Err(AppError::UnexpectedReply),
    }
}

#[tauri::command]
pub async fn get_teachers(state: State<'_, AppState>) -> AppResult<Vec<Teacher>> {
    match state.request(ClientMsgType::GetTeachers).await? {
        ServerMsgType::Teachers(teachers) => Ok(teachers),
        _ => Err(AppError::UnexpectedReply),
    }
}

#[tauri::command]
pub async fn get_activities(state: State<'_, AppState>) -> AppResult<Vec<Activity>> {
    match state.request(ClientMsgType::GetActivities).await? {
        ServerMsgType::Activities(activities) => Ok(activities),
        _ => Err(AppError::UnexpectedReply),
    }
}

// --- Changes ---

#[tauri::command]
pub async fn add_student(
    state: State<'_, AppState>,
    student_name: String,
    key_stage: KeyStage,
) -> AppResult<()> {
    let msg = ClientMsgType::AddStudent {
        student_name,
        key_stage,
    };
    expect_ack(state.request(msg).await?)
}

#[tauri::command]
pub async fn add_activity(
    state: State<'_, AppState>,
    student_id: i64,
    reward_description: String,
    star_diff: i16,
    kind: ActivityKind,
    subject: Subject,
) -> AppResult<()> {
    // The server records it under the logged-in teacher
    let msg = ClientMsgType::AddActivity {
        student_id,
        reward_description,
        star_diff,
        kind,
        subject,
    };
    expect_ack(state.request(msg).await?)
}

#[tauri::command]
pub async fn remove_activity(state: State<'_, AppState>, activity_id: String) -> AppResult<()> {
    expect_ack(state.request(ClientMsgType::RemoveActivity { activity_id }).await?)
}

#[tauri::command]
pub async fn remove_student(state: State<'_, AppState>, student_id: i64) -> AppResult<()> {
    expect_ack(state.request(ClientMsgType::RemoveStudent { student_id }).await?)
}
