mod activities;
mod error;
mod students;
mod teachers;

use std::str::FromStr;

use axum::http::StatusCode;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

pub use activities::{ActivityService, CreateActivity};
pub use error::AppDatabaseError;
pub use students::StudentService;
pub use teachers::TeacherService;

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

    /// A fresh, empty database that only lives in memory, for tests
    #[cfg(test)]
    pub async fn in_memory() -> Self {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")
            .expect("invalid database url")
            .foreign_keys(true);

        // Every connection to `:memory:` opens its own empty database, so keep
        // exactly one connection and never let the pool close it
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None::<std::time::Duration>)
            .max_lifetime(None::<std::time::Duration>)
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

#[cfg(test)]
mod tests {
    use star_chart_protocol::KeyStage;

    use super::*;

    /// A database with one teacher and one student, and their ids
    async fn seeded_db() -> (StarChartDb, i64, i64) {
        let db = StarChartDb::in_memory().await;
        let teacher = db.teachers.create("teacher1", "hash").await.unwrap();
        let student = db.students.create("Alex Smith", KeyStage::Ks3).await.unwrap();
        (db, teacher.id, student.id)
    }

    fn activity(student_id: i64, teacher_id: i64, star_diff: i16) -> CreateActivity {
        CreateActivity {
            student_id,
            teacher_id,
            reward_description: "Great homework".to_string(),
            star_diff,
        }
    }

    async fn total_stars(db: &StarChartDb, student_id: i64) -> i64 {
        db.students
            .list()
            .await
            .unwrap()
            .into_iter()
            .find(|s| s.id == student_id)
            .expect("student should exist")
            .total_stars
    }

    #[tokio::test]
    async fn new_student_starts_with_no_stars() {
        let db = StarChartDb::in_memory().await;
        let student = db.students.create("Alex Smith", KeyStage::Ks4).await.unwrap();

        assert_eq!(student.name, "Alex Smith");
        assert_eq!(student.ks, KeyStage::Ks4);
        assert_eq!(student.total_stars, 0);
    }

    #[tokio::test]
    async fn students_are_listed_in_id_order() {
        let db = StarChartDb::in_memory().await;
        db.students.create("Zoe", KeyStage::Ks3).await.unwrap();
        db.students.create("Adam", KeyStage::Ks4).await.unwrap();

        let names: Vec<String> = db.students.list().await.unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(names, ["Zoe", "Adam"]);
    }

    #[tokio::test]
    async fn duplicate_teacher_name_is_a_conflict() {
        let db = StarChartDb::in_memory().await;
        db.teachers.create("teacher1", "hash").await.unwrap();

        let err = db.teachers.create("teacher1", "hash").await.unwrap_err();
        assert!(matches!(err, AppDatabaseError::Conflict { .. }));
    }

    #[tokio::test]
    async fn password_hash_is_looked_up_by_name() {
        let db = StarChartDb::in_memory().await;
        db.teachers.create("teacher1", "stored-hash").await.unwrap();

        let found = db.teachers.get_password_hash("teacher1").await.unwrap();
        assert_eq!(found.as_deref(), Some("stored-hash"));

        let missing = db.teachers.get_password_hash("nobody").await.unwrap();
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn activity_changes_student_total() {
        let (db, teacher_id, student_id) = seeded_db().await;

        let (activity_row, student) = db
            .activities
            .create(activity(student_id, teacher_id, 5))
            .await
            .unwrap();
        assert_eq!(activity_row.student_id, student_id);
        assert_eq!(activity_row.teacher_id, teacher_id);
        assert_eq!(activity_row.star_diff, 5);
        assert_eq!(student.total_stars, 5);

        // Negative values cash stars in
        let (_, student) = db
            .activities
            .create(activity(student_id, teacher_id, -2))
            .await
            .unwrap();
        assert_eq!(student.total_stars, 3);
        assert_eq!(total_stars(&db, student_id).await, 3);
    }

    #[tokio::test]
    async fn activity_for_unknown_student_writes_nothing() {
        let (db, teacher_id, _) = seeded_db().await;

        let err = db.activities.create(activity(99, teacher_id, 5)).await.unwrap_err();
        assert!(matches!(err, AppDatabaseError::NotFound { entity: "student", .. }));
        assert!(db.activities.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn activity_for_unknown_teacher_writes_nothing() {
        let (db, _, student_id) = seeded_db().await;

        let err = db.activities.create(activity(student_id, 99, 5)).await.unwrap_err();
        assert!(matches!(err, AppDatabaseError::NotFound { entity: "teacher", .. }));
        assert_eq!(total_stars(&db, student_id).await, 0);
        assert!(db.activities.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn activities_are_listed_newest_first() {
        let (db, teacher_id, student_id) = seeded_db().await;
        db.activities.create(activity(student_id, teacher_id, 1)).await.unwrap();
        db.activities.create(activity(student_id, teacher_id, 2)).await.unwrap();

        let diffs: Vec<i16> = db.activities.list().await.unwrap().into_iter().map(|a| a.star_diff).collect();
        assert_eq!(diffs, [2, 1]);
    }
}
