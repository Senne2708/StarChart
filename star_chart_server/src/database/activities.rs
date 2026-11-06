use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::database::error::AppDatabaseError;
use crate::database::students::Student;

#[derive(Serialize, Clone, FromRow)]
pub struct Activity {
    pub id: String,
    pub student_id: i64,
    pub teacher_id: i64,
    pub reward_description: String,
    pub star_diff: i16,
    pub timestamp: DateTime<Utc>,
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

    pub async fn list(&self) -> Result<Vec<Activity>, AppDatabaseError> {
        Ok(sqlx::query_as::<_, Activity>(
            "SELECT id, student_id, teacher_id, reward_description, star_diff, timestamp FROM activities ORDER BY timestamp DESC"
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get(&self, id: &str) -> Result<Activity, AppDatabaseError> {
        sqlx::query_as::<_, Activity>(
            "SELECT id, student_id, teacher_id, reward_description, star_diff, timestamp FROM activities WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppDatabaseError::not_found("activity", id.to_string()))
    }

    pub async fn create(
        &self,
        input: CreateActivity,
    ) -> Result<(Activity, Student), AppDatabaseError> {
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

        let student = sqlx::query_as::<_, Student>(
            "UPDATE students SET total_stars = total_stars + ? WHERE id = ?
             RETURNING id, name, ks, total_stars",
        )
        .bind(input.star_diff)
        .bind(input.student_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppDatabaseError::not_found("student", input.student_id.to_string()))?;

        let activity = sqlx::query_as::<_, Activity>(
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
        .await?;

        tx.commit().await?;
        Ok((activity, student))
    }
}
