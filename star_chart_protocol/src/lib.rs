use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// --- Data types ---

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "lowercase"))]
#[serde(rename_all = "lowercase")]
pub enum KeyStage {
    Ks3,
    Ks4,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Student {
    pub id: i64,
    pub name: String,
    pub ks: KeyStage,
    pub total_stars: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Teacher {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Activity {
    pub id: String,
    pub student_id: i64,
    pub teacher_id: i64,
    pub reward_description: String,
    pub star_diff: i16,
    pub timestamp: DateTime<Utc>,
}

// --- Messaging ---

// Client to Server

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientMsg {
    pub id: u64,
    pub msg: ClientMsgType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ClientMsgType {
    GetTeachers,
    GetStudents,
    GetActivities,

    AddActivity {
        student_id: i64,
        teacher_id: i64,
        reward_description: String,
        star_diff: i16,
    },
    AddStudent {
        student_name: String,
        key_stage: KeyStage,
    },
}

// Server to Client

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ServerMsg {
    pub id: Option<u64>,
    pub msg: ServerMsgType,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ServerMsgType {
    Error {
        code: ErrorCode,
        message: String,
    },
    Ack,
    /// Sent when the client fell behind and missed broadcasts, so it should re-fetch its state
    Resync,

    Students(Vec<Student>),
    Teachers(Vec<Teacher>),
    Activities(Vec<Activity>),

    StudentAdded(Student),

    ActivityAdded {
        activity: Activity,
        student: Student,
    },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadFormat,
    InvalidInput,
    NotFound,
    Conflict,
    InternalError,
}
