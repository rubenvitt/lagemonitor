//! Vertrag zwischen Server und Browser: reine Daten (serde + ts-rs).
//! `cargo test` erzeugt daraus `frontend/src/contract/*.ts`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Reihenfolge = Schwere; `Ord` wird für den Schwellenvergleich genutzt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/contract/")]
pub enum Severity {
    Unknown,
    Minor,
    Moderate,
    Severe,
    Extreme,
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
