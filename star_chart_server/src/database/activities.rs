use chrono::Utc;
use sqlx::SqlitePool;
use star_chart_protocol::{Activity, ActivityKind, Student, Subject};
use uuid::Uuid;

use crate::database::{
    error::AppDatabaseError,
    students::{class_balance, load_student},
};

pub struct CreateActivity {
    pub student_id: i64,
    pub teacher_id: i64,
    pub reward_description: String,
    pub star_diff: i16,
    pub kind: ActivityKind,
    pub subject: Subject,
}

#[derive(Clone)]
pub struct ActivityService {
    pool: SqlitePool,
}

impl ActivityService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<Activity>, AppDatabaseError> {
        Ok(sqlx::query_as::<_, Activity>(
            "SELECT id, student_id, teacher_id, reward_description, star_diff, timestamp, kind, subject
             FROM activities ORDER BY timestamp DESC",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get(&self, id: &str) -> Result<Activity, AppDatabaseError> {
        sqlx::query_as::<_, Activity>(
            "SELECT id, student_id, teacher_id, reward_description, star_diff, timestamp, kind, subject
             FROM activities WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppDatabaseError::not_found("activity", id.to_string()))
    }

    /// Records a star change and returns it with the student's updated totals.
    ///
    /// Fails with `NotEnoughStars` if it would take the student's balance in that
    /// class below zero.
    pub async fn create(
        &self,
        input: CreateActivity,
    ) -> Result<(Activity, Student), AppDatabaseError> {
        // IMMEDIATE takes the write lock up front, so concurrent changes queue
        // instead of failing with SQLITE_BUSY
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;

        let teacher = sqlx::query("SELECT 1 FROM teachers WHERE id = ?")
            .bind(input.teacher_id)
            .fetch_optional(&mut *tx)
            .await?;
        if teacher.is_none() {
            return Err(AppDatabaseError::not_found("teacher", input.teacher_id.to_string()));
        }

        let student = sqlx::query("SELECT 1 FROM students WHERE id = ?")
            .bind(input.student_id)
            .fetch_optional(&mut *tx)
            .await?;
        if student.is_none() {
            return Err(AppDatabaseError::not_found("student", input.student_id.to_string()));
        }

        let activity = sqlx::query_as::<_, Activity>(
            "INSERT INTO activities
                 (id, student_id, teacher_id, reward_description, star_diff, timestamp, kind, subject)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id, student_id, teacher_id, reward_description, star_diff, timestamp, kind, subject",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(input.student_id)
        .bind(input.teacher_id)
        .bind(&input.reward_description)
        .bind(input.star_diff)
        .bind(Utc::now())
        .bind(input.kind)
        .bind(input.subject)
        .fetch_one(&mut *tx)
        .await?;

        // Returning here drops `tx` without committing, which rolls the insert back
        if class_balance(&mut tx, input.student_id, input.subject).await? < 0 {
            return Err(AppDatabaseError::NotEnoughStars {
                student_id: input.student_id,
                subject: input.subject,
            });
        }

        let student = load_student(&mut tx, input.student_id).await?;
        tx.commit().await?;
        Ok((activity, student))
    }

    /// Deletes an activity, which undoes its star change. Returns the updated student.
    ///
    /// Fails with `NotEnoughStars` if that would take the student's balance in the
    /// activity's class below zero, e.g. removing a reward whose stars were cashed in.
    pub async fn remove(&self, id: &str) -> Result<Student, AppDatabaseError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;

        let (student_id, subject): (i64, Subject) =
            sqlx::query_as("DELETE FROM activities WHERE id = ? RETURNING student_id, subject")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| AppDatabaseError::not_found("activity", id))?;

        // Returning here drops `tx` without committing, which rolls the delete back
        if class_balance(&mut tx, student_id, subject).await? < 0 {
            return Err(AppDatabaseError::NotEnoughStars { student_id, subject });
        }

        let student = load_student(&mut tx, student_id).await?;
        tx.commit().await?;
        Ok(student)
    }
}
