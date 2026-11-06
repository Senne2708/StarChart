use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

use crate::database::error::AppDatabaseError;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum KeyStage {
    Ks3,
    Ks4,
}

#[derive(Serialize, Clone, FromRow)]
pub struct Student {
    pub id: i64,
    pub name: String,
    pub ks: KeyStage,
    pub total_stars: i64,
}

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
