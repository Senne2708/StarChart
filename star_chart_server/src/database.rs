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
    use star_chart_protocol::{ActivityKind, KeyStage, Student, Subject};

    use super::*;

    /// A database with one teacher and one student, and their ids
    async fn seeded_db() -> (StarChartDb, i64, i64) {
        let db = StarChartDb::in_memory().await;
        let teacher = db.teachers.create("teacher1", "hash").await.unwrap();
        let student = db.students.create("Alex Smith", KeyStage::Ks3).await.unwrap();
        (db, teacher.id, student.id)
    }

    /// A Maths reward for positive `star_diff`, a Maths take-away for negative
    fn activity(student_id: i64, teacher_id: i64, star_diff: i16) -> CreateActivity {
        let kind = if star_diff < 0 { ActivityKind::TakeAway } else { ActivityKind::Reward };
        star_change(student_id, teacher_id, star_diff, kind, Subject::Maths)
    }

    fn star_change(
        student_id: i64,
        teacher_id: i64,
        star_diff: i16,
        kind: ActivityKind,
        subject: Subject,
    ) -> CreateActivity {
        CreateActivity {
            student_id,
            teacher_id,
            reward_description: "Great homework".to_string(),
            star_diff,
            kind,
            subject,
        }
    }

    async fn student(db: &StarChartDb, student_id: i64) -> Student {
        db.students.get(student_id).await.unwrap()
    }

    #[tokio::test]
    async fn new_student_starts_with_no_stars_in_every_class() {
        let db = StarChartDb::in_memory().await;
        let student = db.students.create("Alex Smith", KeyStage::Ks4).await.unwrap();

        assert_eq!(student.name, "Alex Smith");
        assert_eq!(student.ks, KeyStage::Ks4);
        assert_eq!(student.earned_stars, 0);
        assert_eq!(student.balances.len(), Subject::ALL.len());
        assert!(student.balances.values().all(|&b| b == 0));

        // Loading it back gives the same thing
        assert_eq!(db.students.get(student.id).await.unwrap(), student);
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
    async fn balances_are_kept_per_class_and_earned_ignores_cash_ins() {
        let (db, teacher_id, student_id) = seeded_db().await;

        let (activity_row, student) = db
            .activities
            .create(star_change(student_id, teacher_id, 5, ActivityKind::Reward, Subject::Maths))
            .await
            .unwrap();
        assert_eq!(activity_row.student_id, student_id);
        assert_eq!(activity_row.teacher_id, teacher_id);
        assert_eq!(activity_row.star_diff, 5);
        assert_eq!(activity_row.kind, ActivityKind::Reward);
        assert_eq!(activity_row.subject, Subject::Maths);
        assert_eq!(student.balances[&Subject::Maths], 5);
        assert_eq!(student.earned_stars, 5);

        db.activities
            .create(star_change(student_id, teacher_id, 3, ActivityKind::Reward, Subject::Art))
            .await
            .unwrap();
        let (_, student) = db
            .activities
            .create(star_change(student_id, teacher_id, -2, ActivityKind::CashIn, Subject::Maths))
            .await
            .unwrap();
        assert_eq!(student.balances[&Subject::Maths], 3);
        assert_eq!(student.balances[&Subject::Art], 3);
        assert_eq!(student.balances[&Subject::English], 0);
        assert_eq!(student.earned_stars, 8);

        // A take-away lowers both the class balance and earned
        let (_, student) = db
            .activities
            .create(star_change(student_id, teacher_id, -1, ActivityKind::TakeAway, Subject::Art))
            .await
            .unwrap();
        assert_eq!(student.balances[&Subject::Art], 2);
        assert_eq!(student.earned_stars, 7);

        // Listing works the totals out the same way
        assert_eq!(db.students.list().await.unwrap()[0], student);

        // The kind and class are stored, not just echoed back
        let stored: Vec<(ActivityKind, Subject)> =
            db.activities.list().await.unwrap().into_iter().map(|a| (a.kind, a.subject)).collect();
        assert_eq!(stored.len(), 4);
        for expected in [
            (ActivityKind::Reward, Subject::Maths),
            (ActivityKind::Reward, Subject::Art),
            (ActivityKind::CashIn, Subject::Maths),
            (ActivityKind::TakeAway, Subject::Art),
        ] {
            assert!(stored.contains(&expected), "missing {expected:?}");
        }
    }

    #[tokio::test]
    async fn cannot_spend_one_classes_stars_in_another() {
        let (db, teacher_id, student_id) = seeded_db().await;
        db.activities
            .create(star_change(student_id, teacher_id, 5, ActivityKind::Reward, Subject::Maths))
            .await
            .unwrap();

        let err = db
            .activities
            .create(star_change(student_id, teacher_id, -1, ActivityKind::CashIn, Subject::Art))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            AppDatabaseError::NotEnoughStars { student_id: id, subject: Subject::Art } if id == student_id
        ));
        assert_eq!(err.client_message(), format!("student {student_id} doesn't have enough Art stars"));
        assert_eq!(db.activities.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn overdraw_writes_nothing() {
        let (db, teacher_id, student_id) = seeded_db().await;
        db.activities.create(activity(student_id, teacher_id, 2)).await.unwrap();

        let err = db
            .activities
            .create(star_change(student_id, teacher_id, -3, ActivityKind::CashIn, Subject::Maths))
            .await
            .unwrap_err();
        assert!(matches!(err, AppDatabaseError::NotEnoughStars { .. }));
        assert_eq!(student(&db, student_id).await.balances[&Subject::Maths], 2);
        assert_eq!(db.activities.list().await.unwrap().len(), 1);

        // Spending exactly what they have is fine
        let (_, updated) = db
            .activities
            .create(star_change(student_id, teacher_id, -2, ActivityKind::CashIn, Subject::Maths))
            .await
            .unwrap();
        assert_eq!(updated.balances[&Subject::Maths], 0);
        assert_eq!(updated.earned_stars, 2);
    }

    #[tokio::test]
    async fn removing_an_activity_undoes_its_stars() {
        let (db, teacher_id, student_id) = seeded_db().await;
        let (reward, _) = db.activities.create(activity(student_id, teacher_id, 5)).await.unwrap();
        let (take_away, _) = db.activities.create(activity(student_id, teacher_id, -2)).await.unwrap();

        // Undoing a take-away gives the stars back
        let updated = db.activities.remove(&take_away.id).await.unwrap();
        assert_eq!(updated.balances[&Subject::Maths], 5);
        assert_eq!(updated.earned_stars, 5);

        let updated = db.activities.remove(&reward.id).await.unwrap();
        assert_eq!(updated.balances[&Subject::Maths], 0);
        assert_eq!(updated.earned_stars, 0);
        assert!(db.activities.list().await.unwrap().is_empty());

        let err = db.activities.remove(&reward.id).await.unwrap_err();
        assert!(matches!(err, AppDatabaseError::NotFound { entity: "activity", .. }));
    }

    #[tokio::test]
    async fn removing_a_spent_reward_writes_nothing() {
        let (db, teacher_id, student_id) = seeded_db().await;
        let (reward, _) = db.activities.create(activity(student_id, teacher_id, 5)).await.unwrap();
        db.activities
            .create(star_change(student_id, teacher_id, -4, ActivityKind::CashIn, Subject::Maths))
            .await
            .unwrap();

        // Undoing the +5 would leave them on -4 in Maths
        let err = db.activities.remove(&reward.id).await.unwrap_err();
        assert!(matches!(err, AppDatabaseError::NotEnoughStars { subject: Subject::Maths, .. }));
        assert_eq!(student(&db, student_id).await.balances[&Subject::Maths], 1);
        assert_eq!(db.activities.list().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn removing_is_only_blocked_by_that_classes_balance() {
        let (db, teacher_id, student_id) = seeded_db().await;
        let (art_reward, _) = db
            .activities
            .create(star_change(student_id, teacher_id, 2, ActivityKind::Reward, Subject::Art))
            .await
            .unwrap();
        db.activities.create(activity(student_id, teacher_id, 4)).await.unwrap();
        db.activities
            .create(star_change(student_id, teacher_id, -4, ActivityKind::CashIn, Subject::Maths))
            .await
            .unwrap();

        // Maths is spent, but Art isn't, so the Art reward can go
        let updated = db.activities.remove(&art_reward.id).await.unwrap();
        assert_eq!(updated.balances[&Subject::Art], 0);
        assert_eq!(updated.balances[&Subject::Maths], 0);
        assert_eq!(updated.earned_stars, 4);
    }

    #[tokio::test]
    async fn removing_a_student_removes_their_activities() {
        let (db, teacher_id, student_id) = seeded_db().await;
        let other = db.students.create("Sam Jones", KeyStage::Ks4).await.unwrap();
        db.activities.create(activity(student_id, teacher_id, 5)).await.unwrap();
        db.activities.create(activity(other.id, teacher_id, 3)).await.unwrap();

        db.students.remove(student_id).await.unwrap();

        let students = db.students.list().await.unwrap();
        assert_eq!(students.len(), 1);
        assert_eq!(students[0].id, other.id);
        assert_eq!(students[0].earned_stars, 3);
        let activities = db.activities.list().await.unwrap();
        assert_eq!(activities.len(), 1);
        assert_eq!(activities[0].student_id, other.id);

        let err = db.students.remove(student_id).await.unwrap_err();
        assert!(matches!(err, AppDatabaseError::NotFound { entity: "student", .. }));
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
        assert_eq!(student(&db, student_id).await.earned_stars, 0);
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
