//! Fachliche Einstellungen aus der Tabelle `settings` (Wert je Schlüssel als JSON).

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::contract::{Location, Modules, Notice, Recruitment, Region, Severity};
use crate::error::AppError;

/// Verbindung zu Suchlage: Adresse und Großbild-Token, dazu, was vom Einsatz erscheint.
/// Bleibt auf dem Server; der Browser sieht den Token nie.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuchlageLink {
    pub url: String,
    pub token: String,
    pub tracks: bool,
    pub areas: bool,
    pub video: bool,
}

/// Ein Link, mit dem ein Rechner den Monitor ohne Pocket ID sieht (`auth::link`). Angelegt und
/// widerrufen in der Suite; hier steht nur der SHA-256 des Schlüssels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicLink {
    pub id: String,
    pub label: String,
    pub hash: String,
}

/// Verkehrslage: welche Ebenen an sind, dazu der TomTom-Schlüssel. Der Schlüssel bleibt auf dem
/// Server; Kacheln und Störungen holt der Server (`web::traffic`, `poller::poll_traffic`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficLink {
    pub tomtom_key: Option<String>,
    pub flow: bool,
    pub incidents: bool,
    pub speed_cameras: bool,
}

impl TrafficLink {
    /// Der Schlüssel, wenn eine TomTom-Ebene ihn braucht und er da ist.
    pub fn key_for(&self, layer_on: bool) -> Option<&str> {
        self.tomtom_key.as_deref().filter(|_| layer_on)
    }
}

/// Zoomstufe ohne Einsatz, solange die Suite keine schickt: etwa der Landkreis.
pub const MAP_ZOOM_DEFAULT: f64 = 9.0;
/// Was die Suite einstellen darf; außerhalb gilt der nächste Rand.
pub const MAP_ZOOM_MIN: f64 = 5.0;
pub const MAP_ZOOM_MAX: f64 = 16.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub location: Location,
    pub region: Region,
    pub warn_threshold: Severity,
    pub modules: Modules,
    pub suchlage: Option<SuchlageLink>,
    /// Meldungen fürs Band; auch abgelaufene bleiben bis zur nächsten Abholung hier stehen.
    pub notices: Vec<Notice>,
    pub recruitment: Option<Recruitment>,
    pub public_links: Vec<PublicLink>,
    pub traffic: TrafficLink,
    /// Zoomstufe der Karte ohne Einsatz.
    pub map_zoom: f64,
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
        modules: get(pool, "modules").await?.unwrap_or_default(),
        suchlage: get(pool, "suchlage").await?.flatten(),
        notices: get(pool, "notices").await?.unwrap_or_default(),
        recruitment: get(pool, "recruitment").await?.flatten(),
        public_links: get(pool, "public_links").await?.unwrap_or_default(),
        traffic: get(pool, "traffic").await?.unwrap_or_default(),
        map_zoom: get(pool, "map_zoom").await?.unwrap_or(MAP_ZOOM_DEFAULT),
    })
}

/// Schreibt alle Einstellungen in einem Zug; ein Leser sieht nie eine halbe Konfiguration.
pub async fn save(pool: &SqlitePool, s: &Settings) -> Result<(), AppError> {
    let rows = [
        ("location", to_json(&s.location)?),
        ("region", to_json(&s.region)?),
        ("warn_threshold", to_json(&s.warn_threshold)?),
        ("modules", to_json(&s.modules)?),
        ("suchlage", to_json(&s.suchlage)?),
        ("notices", to_json(&s.notices)?),
        ("recruitment", to_json(&s.recruitment)?),
        ("public_links", to_json(&s.public_links)?),
        ("traffic", to_json(&s.traffic)?),
        ("map_zoom", to_json(&s.map_zoom)?),
    ];
    let mut tx = pool.begin().await?;
    for (key, value) in rows {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

fn to_json(value: &impl Serialize) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(|e| AppError::Internal(e.to_string()))
}

async fn get<T: DeserializeOwned>(
    pool: &SqlitePool,
    key: &'static str,
) -> Result<Option<T>, AppError> {
    let raw: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    raw.map(|json| {
        serde_json::from_str(&json).map_err(|e| {
            tracing::warn!(key, error = %e, "Einstellung nicht lesbar");
            AppError::Settings(key)
        })
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{NoticeCategory, NoticeColor, NoticeIcon};

    #[tokio::test]
    async fn defaults_when_table_empty() {
        let pool = crate::db::test_pool().await;
        let s = load(&pool).await.unwrap();
        assert_eq!(s.location.label, "Uelzen");
        assert_eq!(s.region.ars, "033600000000");
        assert_eq!(s.warn_threshold, Severity::Severe);
        assert_eq!(s.modules, Modules::default());
        assert_eq!(s.suchlage, None);
        assert!(s.notices.is_empty());
        assert_eq!(s.recruitment, None);
        assert!(s.public_links.is_empty());
        assert_eq!(s.traffic, TrafficLink::default());
        assert_eq!(s.map_zoom, MAP_ZOOM_DEFAULT);
    }

    #[tokio::test]
    async fn save_and_load_roundtrip() {
        let pool = crate::db::test_pool().await;
        let mut s = load(&pool).await.unwrap();
        s.modules.radar = false;
        s.suchlage = Some(SuchlageLink {
            url: "http://127.0.0.1:8090".into(),
            token: "geheim".into(),
            tracks: true,
            areas: false,
            video: true,
        });
        s.warn_threshold = Severity::Minor;
        s.notices = vec![Notice {
            id: "n1".into(),
            text: "Nächste Lagebesprechung".into(),
            countdown_to_ms: Some(1_800_000_000_000),
            expires_at_ms: 1_800_000_600_000,
            category: Some(NoticeCategory {
                name: "Besprechung".into(),
                color: NoticeColor::Purple,
                icon: Some(NoticeIcon {
                    view_box: "0 0 48 48".into(),
                    paths: vec!["M14 4z".into()],
                }),
                alert_lead_minutes: Some(10),
            }),
        }];
        s.recruitment = Some(Recruitment {
            url: "https://drk.example.org/mitmachen".into(),
            text: "Lust mitzuhelfen?".into(),
        });
        s.public_links = vec![PublicLink {
            id: "l1".into(),
            label: "Rechner Wache".into(),
            hash: "ab".repeat(32),
        }];
        s.traffic = TrafficLink {
            tomtom_key: Some("schluessel".into()),
            flow: true,
            incidents: false,
            speed_cameras: true,
        };
        s.map_zoom = 11.5;
        save(&pool, &s).await.unwrap();
        assert_eq!(load(&pool).await.unwrap(), s);
        s.suchlage = None;
        save(&pool, &s).await.unwrap();
        assert_eq!(load(&pool).await.unwrap().suchlage, None);
    }

    #[tokio::test]
    async fn threshold_ignores_case() {
        let pool = crate::db::test_pool().await;
        sqlx::query("INSERT INTO settings (key, value) VALUES ('warn_threshold', '\"moderate\"')")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            load(&pool).await.unwrap().warn_threshold,
            Severity::Moderate
        );
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
