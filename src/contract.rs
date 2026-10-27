//! Vertrag zwischen Server und Browser: reine Daten (serde + ts-rs).
//! `cargo test` erzeugt daraus `frontend/src/contract/*.ts`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Reihenfolge = Schwere; `Ord` wird für den Schwellenvergleich genutzt.
/// Gelesen wird ohne Rücksicht auf Groß-/Kleinschreibung (`"severe"` in den Einstellungen, `"SEVERE"` bei NINA).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/contract/")]
pub enum Severity {
    Unknown,
    Minor,
    Moderate,
    Severe,
    Extreme,
}

impl Severity {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name.to_ascii_lowercase().as_str() {
            "unknown" => Severity::Unknown,
            "minor" => Severity::Minor,
            "moderate" => Severity::Moderate,
            "severe" => Severity::Severe,
            "extreme" => Severity::Extreme,
            _ => return None,
        })
    }
}

impl<'de> Deserialize<'de> for Severity {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Self::from_name(&name).ok_or_else(|| {
            serde::de::Error::unknown_variant(
                &name,
                &["Unknown", "Minor", "Moderate", "Severe", "Extreme"],
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct Region {
    pub ars: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub enum Provider {
    Dwd,
    Mowas,
    Katwarn,
    Biwapp,
    Lhp,
    Police,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct Warning {
    pub id: String,
    pub provider: Provider,
    pub event: String,
    pub headline: String,
    pub description: String,
    pub instruction: Option<String>,
    pub severity: Severity,
    pub level_label: String,
    pub onset: Option<String>,
    pub expires: Option<String>,
    pub sender_name: Option<String>,
    /// GeoJSON-FeatureCollection; `null`, wenn NINA keine Geometrie liefert.
    #[ts(type = "unknown")]
    pub geometry: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct CurrentWeather {
    pub timestamp: String,
    pub temperature: Option<f64>,
    pub condition: Option<String>,
    pub icon: Option<String>,
    pub wind_speed: Option<f64>,
    pub wind_gust_speed: Option<f64>,
    pub wind_direction: Option<f64>,
    pub precipitation: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct HourlyWeather {
    pub timestamp: String,
    pub temperature: Option<f64>,
    pub icon: Option<String>,
    pub precipitation: Option<f64>,
}

/// `hourly`: die nächsten 12 Stunden ab jetzt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct Weather {
    pub current: CurrentWeather,
    pub hourly: Vec<HourlyWeather>,
}

/// Ein Datenstand samt Alter. `data: null` heißt: noch nie etwas empfangen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct Sourced<T> {
    pub data: Option<T>,
    // JSON liefert eine Zahl; ts-rs würde sonst `bigint` erzeugen.
    #[ts(type = "number | null")]
    pub fetched_at_ms: Option<i64>,
    pub stale: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct MapInfo {
    /// `true`, wenn die Offline-Kacheln (`karte.mbtiles`) geladen sind.
    pub offline: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct MonitorView {
    #[ts(type = "number")]
    pub now_ms: i64,
    pub location: Location,
    pub region: Region,
    pub warn_threshold: Severity,
    pub weather: Sourced<Weather>,
    pub warnings: Sourced<Vec<Warning>>,
    pub map: MapInfo,
}
