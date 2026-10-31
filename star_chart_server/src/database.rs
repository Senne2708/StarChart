mod activities;
mod error;
mod students;
mod teachers;
 
use std::str::FromStr;
 
use axum::http::StatusCode;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;
 
pub use activities::{Activities, ActivityService, CreateActivity};
pub use error::AppDatabaseError;
pub use students::{Student, StudentService};
pub use teachers::{Teacher, TeacherService};

pub fn db_error(e: AppDatabaseError) -> StatusCode {
    tracing::error!(?e, "database error");
    StatusCode::INTERNAL_SERVER_ERROR
}

pub struct StarChartDb {
    pub students: StudentService,
    pub teachers: TeacherService,
    pub activities: ActivityService,
}

impl StarChartDb {
    pub async fn connect() -> Self {
        let options = SqliteConnectOptions::from_str("sqlite:app.db")
            .expect("invalid database url")
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal);
        
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .expect("failed to open database");

        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("failed to run migrations");

        Self::new(pool)
    }

    fn new(pool: SqlitePool) -> Self {
        Self {
            students: StudentService::new(pool.clone()),
            teachers: TeacherService::new(pool.clone()),
            activities: ActivityService::new(pool.clone()),
        }
    }
}
