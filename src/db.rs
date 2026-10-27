//! SQLite-Anschluss nach Suchlage `db.rs`: WAL, busy_timeout 5 s, Fremdschlüssel.

use std::time::Duration;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use crate::config::BootConfig;
use crate::error::AppError;

pub async fn connect(boot: &BootConfig) -> Result<SqlitePool, AppError> {
    std::fs::create_dir_all(&boot.data_dir)
        .map_err(|e| AppError::Internal(format!("Datenordner: {e}")))?;
    let options = SqliteConnectOptions::new()
        .filename(boot.db_path())
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(options)
        .await?;
    migrate(&pool).await?;
    Ok(pool)
}

async fn migrate(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::Internal(format!("Migration: {e}")))
}

/// Frische In-Memory-DB je Test. Eine Verbindung, sonst sähe jede ihre eigene leere DB.
#[cfg(test)]
pub async fn test_pool() -> SqlitePool {
    let options = SqliteConnectOptions::new().in_memory(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    migrate(&pool).await.unwrap();
    pool
}
