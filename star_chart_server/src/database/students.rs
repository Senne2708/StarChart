use std::collections::{BTreeMap, HashMap};

use sqlx::{SqliteConnection, SqlitePool};
use star_chart_protocol::{KeyStage, Student, Subject};

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
        let mut conn = self.pool.acquire().await?;
        load_students(&mut conn, None).await
    }

    pub async fn get(&self, id: i64) -> Result<Student, AppDatabaseError> {
        let mut conn = self.pool.acquire().await?;
        load_student(&mut conn, id).await
    }

    pub async fn create(
        &self,
        name: &str,
        key_stage: KeyStage,
    ) -> Result<Student, AppDatabaseError> {
        let (id, name, ks): (i64, String, KeyStage) =
            sqlx::query_as("INSERT INTO students (name, ks) VALUES (?, ?) RETURNING id, name, ks")
                .bind(name)
                .bind(key_stage)
                .fetch_one(&self.pool)
                .await?;

        // A new student has no activities yet
        Ok(Student {
            id,
            name,
            ks,
            balances: zero_balances(),
            earned_stars: 0,
        })
    }

    /// Deletes a student and all of their activities
    pub async fn remove(&self, id: i64) -> Result<(), AppDatabaseError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM activities WHERE student_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        let deleted = sqlx::query("DELETE FROM students WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        if deleted.rows_affected() == 0 {
            return Err(AppDatabaseError::not_found("student", id.to_string()));
        }

        tx.commit().await?;
        Ok(())
    }
}

fn zero_balances() -> BTreeMap<Subject, i64> {
    Subject::ALL.iter().map(|&subject| (subject, 0)).collect()
}

/// Builds students with their star totals worked out from their activities, in id order.
/// `only` limits it to one student. Takes a connection so it also works inside a transaction.
pub(crate) async fn load_students(
    conn: &mut SqliteConnection,
    only: Option<i64>,
) -> Result<Vec<Student>, AppDatabaseError> {
    let rows: Vec<(i64, String, KeyStage)> =
        sqlx::query_as("SELECT id, name, ks FROM students WHERE ? IS NULL OR id = ? ORDER BY id")
            .bind(only)
            .bind(only)
            .fetch_all(&mut *conn)
            .await?;

    let balances: Vec<(i64, Subject, i64)> = sqlx::query_as(
        "SELECT student_id, subject, SUM(star_diff) FROM activities
         WHERE ? IS NULL OR student_id = ?
         GROUP BY student_id, subject",
    )
    .bind(only)
    .bind(only)
    .fetch_all(&mut *conn)
    .await?;

    let earned: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT student_id, SUM(star_diff) FROM activities
         WHERE (? IS NULL OR student_id = ?) AND kind != 'cash_in'
         GROUP BY student_id",
    )
    .bind(only)
    .bind(only)
    .fetch_all(&mut *conn)
    .await?;

    let mut students: Vec<Student> = rows
        .into_iter()
        .map(|(id, name, ks)| Student {
            id,
            name,
            ks,
            balances: zero_balances(),
            earned_stars: 0,
        })
        .collect();
    let position: HashMap<i64, usize> = students.iter().enumerate().map(|(i, s)| (s.id, i)).collect();

    for (student_id, subject, total) in balances {
        if let Some(&i) = position.get(&student_id) {
            students[i].balances.insert(subject, total);
        }
    }
    for (student_id, total) in earned {
        if let Some(&i) = position.get(&student_id) {
            students[i].earned_stars = total;
        }
    }

    Ok(students)
}

/// One student with their star totals, or `NotFound`
pub(crate) async fn load_student(
    conn: &mut SqliteConnection,
    id: i64,
) -> Result<Student, AppDatabaseError> {
    load_students(conn, Some(id))
        .await?
        .pop()
        .ok_or_else(|| AppDatabaseError::not_found("student", id.to_string()))
}

/// A student's balance in one class
pub(crate) async fn class_balance(
    conn: &mut SqliteConnection,
    student_id: i64,
    subject: Subject,
) -> Result<i64, AppDatabaseError> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(SUM(star_diff), 0) FROM activities WHERE student_id = ? AND subject = ?",
    )
    .bind(student_id)
    .bind(subject)
    .fetch_one(&mut *conn)
    .await?)
}
