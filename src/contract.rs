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
