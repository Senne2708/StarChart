use sqlx::SqlitePool;
use star_chart_protocol::{KeyStage, Student};

use crate::database::error::AppDatabaseError;

#[derive(Clone)]
pub struct StudentService {
    pool: SqlitePool,
}

impl StudentService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<Student>, AppDatabaseError> {
        Ok(sqlx::query_as::<_, Student>(
            "SELECT id, name, ks, total_stars FROM students ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn get(&self, id: i64) -> Result<Student, AppDatabaseError> {
        sqlx::query_as::<_, Student>("SELECT id, name, ks, total_stars FROM students WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppDatabaseError::not_found("student", id.to_string()))
    }

    pub async fn create(
        &self,
        name: &str,
        key_stage: KeyStage,
    ) -> Result<Student, AppDatabaseError> {
        sqlx::query_as::<_, Student>(
            "INSERT INTO students (name, ks) VALUES (?, ?) RETURNING id, name, ks, total_stars",
        )
        .bind(name)
        .bind(key_stage)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }
}
