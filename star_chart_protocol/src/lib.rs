use std::collections::BTreeMap;

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

/// What a star change was for. `Reward` adds stars; `TakeAway` and `CashIn` remove them.
/// Cash-ins lower a student's balance but not their earned score on the leaderboard.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Reward,
    TakeAway,
    CashIn,
}

/// The classes every student takes, in display order. Stars are always given in a class.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(feature = "sqlx", sqlx(rename_all = "snake_case"))]
#[serde(rename_all = "snake_case")]
pub enum Subject {
    Art,
    Ilp,
    Maths,
    English,
    Humanities,
    Citizenship,
    Vocational,
    Options,
}

impl Subject {
    pub const ALL: [Subject; 8] = [
        Subject::Art,
        Subject::Ilp,
        Subject::Maths,
        Subject::English,
        Subject::Humanities,
        Subject::Citizenship,
        Subject::Vocational,
        Subject::Options,
    ];

    /// The name shown to people, e.g. "ILP"
    pub fn label(self) -> &'static str {
        match self {
            Subject::Art => "Art",
            Subject::Ilp => "ILP",
            Subject::Maths => "Maths",
            Subject::English => "English",
            Subject::Humanities => "Humanities",
            Subject::Citizenship => "Citizenship",
            Subject::Vocational => "Vocational",
            Subject::Options => "Options",
        }
    }
}

/// Star totals are worked out from the student's activities, not stored.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Student {
    pub id: i64,
    pub name: String,
    pub ks: KeyStage,
    /// Stars to spend in each class: awarded - taken away - cashed in.
    /// Always has an entry for every class.
    pub balances: BTreeMap<Subject, i64>,
    /// Awarded - taken away, across all classes. Cash-ins don't lower it.
    pub earned_stars: i64,
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
    pub kind: ActivityKind,
    pub subject: Subject,
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

    /// Recorded under the logged-in teacher, taken from the login token
    AddActivity {
        student_id: i64,
        reward_description: String,
        star_diff: i16,
        kind: ActivityKind,
        subject: Subject,
    },
    AddStudent {
        student_name: String,
        key_stage: KeyStage,
    },

    /// Deletes an activity and undoes its star change
    RemoveActivity {
        activity_id: String,
    },
    /// Deletes a student and all of their activities
    RemoveStudent {
        student_id: i64,
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
    Resync,

    Students(Vec<Student>),
    Teachers(Vec<Teacher>),
    Activities(Vec<Activity>),

    StudentAdded(Student),

    ActivityAdded {
        activity: Activity,
        student: Student,
    },

    /// `student` has the star total after the activity's change was undone
    ActivityRemoved {
        activity_id: String,
        student: Student,
    },
    /// The student's activities were removed with them
    StudentRemoved {
        student_id: i64,
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
