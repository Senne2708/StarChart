use serde::{Deserialize, Serialize};

use crate::database::{Activity, Student, Teacher};

// Client to Server

#[derive(Deserialize)]
pub struct ClientMsg {
    pub id: u64,
    pub msg: ClientMsgType,
}

#[derive(Deserialize, Debug)]
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
        key_stage: String,
    },
}

// Server to Client

#[derive(Serialize, Clone)]
pub struct ServerMsg {
    pub id: Option<u64>,
    pub msg: ServerMsgType,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ServerMsgType {
    Error {
        code: &'static str,
        message: String,
    },
    Ack,

    Students(Vec<Student>),
    Teachers(Vec<Teacher>),
    Activities(Vec<Activity>),

    StudentAdded(Student),

    ActivityAdded {
        activity: Activity,
        student: Student,
    },
}
