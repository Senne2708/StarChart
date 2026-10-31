use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::database::error::AppDatabaseError;
use crate::database::students::{Student, StudentRow};

#[derive(Serialize, Clone)]
pub struct Activities {
    pub id: String,
    pub student_id: i64,
    pub teacher_id: i64,
    pub reward_description: String,
    pub star_diff: i16,
    pub timestamp: DateTime<Utc>,
}

#[derive(FromRow)]
struct ActivitiesRow {
    id: String,
    student_id: i64,
    teacher_id: i64,
    reward_description: String,
    star_diff: i16,
    timestamp: DateTime<Utc>,
}

impl ActivitiesRow {
    fn into_activities(self) -> Activities {
        Activities {
            id: self.id,
            student_id: self.student_id,
            teacher_id: self.teacher_id,
            reward_description: self.reward_description,
            star_diff: self.star_diff,
            timestamp: self.timestamp,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateActivity {
    pub student_id: i64,
    pub teacher_id: i64,
    pub reward_description: String,
    pub star_diff: i16,
}

#[derive(Clone)]
pub struct ActivityService {
    pool: SqlitePool,
}

impl ActivityService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<Activities>, AppDatabaseError> {
        let rows = sqlx::query_as::<_, ActivitiesRow>(
            "SELECT id, student_id, teacher_id, reward_description, star_diff, timestamp FROM activities ORDER BY timestamp DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(ActivitiesRow::into_activities)
            .collect())
    }

    pub async fn get(&self, id: &str) -> Result<Activities, AppDatabaseError> {
        sqlx::query_as::<_, ActivitiesRow>(
            "SELECT id, student_id, teacher_id, reward_description, star_diff, timestamp FROM activities WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .map(ActivitiesRow::into_activities)
        .ok_or_else(|| AppDatabaseError::not_found("activity", id.to_string()))
    }

    pub async fn create(
        &self,
        input: CreateActivity,
    ) -> Result<(Activities, Student), AppDatabaseError> {
        let mut tx = self.pool.begin().await?;

        let teacher = sqlx::query("SELECT 1 FROM teachers WHERE id = ?")
            .bind(input.teacher_id)
            .fetch_optional(&mut *tx)
            .await?;
        if teacher.is_none() {
            return Err(AppDatabaseError::not_found(
                "teacher",
                input.teacher_id.to_string(),
            ));
        }

        let student = sqlx::query_as::<_, StudentRow>(
            "UPDATE students SET total_stars = total_stars + ? WHERE id = ?
             RETURNING id, name, total_stars",
        )
        .bind(input.star_diff)
        .bind(input.student_id)
        .fetch_optional(&mut *tx)
        .await?
        .map(StudentRow::into_student)
        .ok_or_else(|| AppDatabaseError::not_found("student", input.student_id.to_string()))?;

        let activity = sqlx::query_as::<_, ActivitiesRow>(
            "INSERT INTO activities
                 (id, student_id, teacher_id, reward_description, star_diff, timestamp)
             VALUES (?, ?, ?, ?, ?, ?)
             RETURNING id, student_id, teacher_id, reward_description, star_diff, timestamp",
        )
        .bind(Uuid::now_v7().to_string())
        .bind(input.student_id)
        .bind(input.teacher_id)
        .bind(&input.reward_description)
        .bind(input.star_diff)
        .bind(Utc::now())
        .fetch_one(&mut *tx)
        .await?
        .into_activities();

        tx.commit().await?;
        Ok((activity, student))
    }
}
