use sqlx::SqlitePool;
use star_chart_protocol::Teacher;

use crate::database::error::AppDatabaseError;

#[derive(Clone)]
pub struct TeacherService {
    pool: SqlitePool,
}

impl TeacherService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<Teacher>, AppDatabaseError> {
        let rows = sqlx::query_as::<_, Teacher>("SELECT id, name FROM teachers ORDER BY id")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    pub async fn get(&self, id: i64) -> Result<Teacher, AppDatabaseError> {
        sqlx::query_as::<_, Teacher>("SELECT id, name FROM teachers WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppDatabaseError::not_found("teacher", id.to_string()))
    }

    pub async fn create(
        &self,
        name: &str,
        hashed_password: &str,
    ) -> Result<Teacher, AppDatabaseError> {
        sqlx::query_as::<_, Teacher>(
            "INSERT INTO teachers (name, hashed_password) VALUES (?, ?) RETURNING id, name",
        )
        .bind(name)
        .bind(hashed_password)
        .fetch_one(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn get_password_hash(&self, name: &str) -> Result<Option<String>, AppDatabaseError> {
        sqlx::query_scalar::<_, String>("SELECT hashed_password FROM teachers WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(Into::into)
    }
}
