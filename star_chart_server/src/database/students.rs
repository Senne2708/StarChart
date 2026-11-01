use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

use crate::database::error::AppDatabaseError;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum KeyStage { 
    Ks3,
    Ks4
}

#[derive(Serialize, Clone)]
pub struct Student {
    pub id: i64,
    pub name: String,
    pub ks: KeyStage,
    pub total_stars: i16,
}

#[derive(FromRow)]
pub struct StudentRow {
    id: i64,
    name: String,
    ks: KeyStage,
    total_stars: i16,
}

impl StudentRow {
    pub fn into_student(self) -> Student {
        Student { id: self.id, name: self.name, ks: self.ks, total_stars: self.total_stars }
    }
}

#[derive(Clone)]
pub struct StudentService {
    pool: SqlitePool,
}

impl StudentService {
    pub fn new(pool: SqlitePool) -> Self { Self { pool } }

    pub async fn list(&self) -> Result<Vec<Student>, AppDatabaseError> {
        let rows = sqlx::query_as::<_, StudentRow>(
            "SELECT id, name, ks, total_stars FROM students ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(StudentRow::into_student).collect())
    }

    pub async fn get(&self, id: i64) -> Result<Student, AppDatabaseError> {
        sqlx::query_as::<_, StudentRow>(
            "SELECT id, name, ks, total_stars FROM students WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .map(StudentRow::into_student)
        .ok_or_else(|| AppDatabaseError::not_found("student", id.to_string()))
    }

    pub async fn create(&self, name: &str, key_stage: &str) -> Result<Student, AppDatabaseError> {
        sqlx::query_as::<_, StudentRow>(
            "INSERT INTO students (name, ks) VALUES (?, ?) RETURNING id, name, ks, total_stars",
        )
        .bind(name)
        .bind(key_stage)
        .fetch_one(&self.pool)
        .await
        .map(StudentRow::into_student)
        .map_err(Into::into)
    }
}
