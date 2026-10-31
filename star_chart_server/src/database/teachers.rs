use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

use crate::database::error::AppDatabaseError;

#[derive(Serialize, Clone)]
pub struct Teacher {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing)]
    pub hashed_password: String,
}

#[derive(FromRow)]
struct TeacherRow {
    id: i64,
    name: String,
    hashed_password: String,
}

impl TeacherRow {
    fn into_teacher(self) -> Teacher {
        Teacher {
            id: self.id,
            name: self.name,
            hashed_password: self.hashed_password,
        }
    }
}

#[derive(Clone)]
pub struct TeacherService {
    pool: SqlitePool,
}

impl TeacherService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<Teacher>, AppDatabaseError> {
        let rows = sqlx::query_as::<_, TeacherRow>("SELECT id, name, hashed_password FROM teachers ORDER BY id")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(TeacherRow::into_teacher).collect())
    }

    pub async fn get(&self, id: i64) -> Result<Teacher, AppDatabaseError> {
        sqlx::query_as::<_, TeacherRow>("SELECT id, name, hashed_password FROM teachers WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .map(TeacherRow::into_teacher)
            .ok_or_else(|| AppDatabaseError::not_found("teacher", id.to_string()))
    }

    pub async fn create(&self, name: &str, hashed_password: &str) -> Result<Teacher, AppDatabaseError> {
        sqlx::query_as::<_, TeacherRow>("INSERT INTO teachers (name, hashed_password) VALUES (?, ?) RETURNING id, name, hashed_password")
            .bind(name)
            .bind(hashed_password)
            .fetch_one(&self.pool)
            .await
            .map(TeacherRow::into_teacher)
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
