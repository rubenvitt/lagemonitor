//! Letzter Stand je Quelle in der Tabelle `snapshots`.

use serde::Serialize;
use serde::de::DeserializeOwned;
use sqlx::SqlitePool;

use crate::error::AppError;

pub async fn save(
    pool: &SqlitePool,
    source: &str,
    fetched_at_ms: i64,
    body: &impl Serialize,
) -> Result<(), AppError> {
    let json = serde_json::to_string(body).map_err(|e| AppError::Internal(e.to_string()))?;
    sqlx::query(
        "INSERT INTO snapshots (source, fetched_at_ms, body) VALUES (?, ?, ?)
         ON CONFLICT(source) DO UPDATE SET fetched_at_ms = excluded.fetched_at_ms, body = excluded.body",
    )
    .bind(source)
    .bind(fetched_at_ms)
    .bind(json)
    .execute(pool)
    .await?;
    Ok(())
}

/// `None` ohne Stand. Ein nicht lesbarer Stand (z. B. nach einer Vertragsänderung) zählt wie keiner.
pub async fn load<T: DeserializeOwned>(pool: &SqlitePool, source: &str) -> Option<(i64, T)> {
    let row: (i64, String) =
        sqlx::query_as("SELECT fetched_at_ms, body FROM snapshots WHERE source = ?")
            .bind(source)
            .fetch_optional(pool)
            .await
            .ok()??;
    match serde_json::from_str(&row.1) {
        Ok(value) => Some((row.0, value)),
        Err(e) => {
            tracing::warn!(source, error = %e, "gespeicherter Stand nicht lesbar");
            None
        }
    }
}

/// Nur der Zeitpunkt des Stands, ohne ihn zu lesen.
pub async fn fetched_at(pool: &SqlitePool, source: &str) -> Option<i64> {
    sqlx::query_scalar("SELECT fetched_at_ms FROM snapshots WHERE source = ?")
        .bind(source)
        .fetch_optional(pool)
        .await
        .ok()?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn store_roundtrip() {
        let pool = crate::db::test_pool().await;
        assert!(load::<Vec<i32>>(&pool, "nina").await.is_none());
        save(&pool, "nina", 1000, &vec![1, 2]).await.unwrap();
        save(&pool, "nina", 2000, &vec![3]).await.unwrap();
        assert_eq!(load::<Vec<i32>>(&pool, "nina").await, Some((2000, vec![3])));
        assert_eq!(fetched_at(&pool, "nina").await, Some(2000));
        assert_eq!(fetched_at(&pool, "wetter").await, None);
    }
}
