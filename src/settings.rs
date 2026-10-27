//! Fachliche Einstellungen aus der Tabelle `settings` (Wert je Schlüssel als JSON).

use serde::de::DeserializeOwned;
use sqlx::SqlitePool;

use crate::contract::{Location, Region, Severity};
use crate::error::AppError;

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub location: Location,
    pub region: Region,
    pub warn_threshold: Severity,
}

pub async fn load(pool: &SqlitePool) -> Result<Settings, AppError> {
    Ok(Settings {
        location: get(pool, "location").await?.unwrap_or_else(|| Location {
            lat: 52.9656,
            lon: 10.5594,
            label: "Uelzen".into(),
        }),
        region: get(pool, "region").await?.unwrap_or_else(|| Region {
            ars: "033600000000".into(),
            name: "Landkreis Uelzen".into(),
        }),
        warn_threshold: get(pool, "warn_threshold")
            .await?
            .unwrap_or(Severity::Severe),
    })
}

async fn get<T: DeserializeOwned>(pool: &SqlitePool, key: &str) -> Result<Option<T>, AppError> {
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    raw.map(|json| {
        serde_json::from_str(&json)
            .map_err(|e| AppError::Internal(format!("Einstellung {key}: {e}")))
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn defaults_when_table_empty() {
        let pool = crate::db::test_pool().await;
        let s = load(&pool).await.unwrap();
        assert_eq!(s.location.label, "Uelzen");
        assert_eq!(s.region.ars, "033600000000");
        assert_eq!(s.warn_threshold, Severity::Severe);
    }

    #[tokio::test]
    async fn stored_value_wins() {
        let pool = crate::db::test_pool().await;
        sqlx::query("INSERT INTO settings (key, value) VALUES ('region', ?)")
            .bind(r#"{"ars":"031550000000","name":"Landkreis Celle"}"#)
            .execute(&pool)
            .await
            .unwrap();
        let s = load(&pool).await.unwrap();
        assert_eq!(s.region.name, "Landkreis Celle");
        assert_eq!(s.location.label, "Uelzen");
    }
}
