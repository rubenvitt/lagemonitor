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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct MapInfo {
    /// `true`, wenn die Offline-Kacheln (`karte.mbtiles`) geladen sind.
    pub offline: bool,
    /// Zoomstufe der Karte ohne Einsatz (um den Standort); eingestellt in der Suite.
    pub zoom: f64,
}

/// Welche Teile der Monitor zeigt. Das Warnband ist kein Modul: Amtliche Warnungen über der
/// Schwelle brechen immer durch (Spec 4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct Modules {
    pub weather: bool,
    pub radar: bool,
    pub warning_areas: bool,
    pub warning_list: bool,
    pub suchlage: bool,
}

impl Default for Modules {
    fn default() -> Self {
        Self {
            weather: true,
            radar: true,
            warning_areas: true,
            warning_list: true,
            suchlage: false,
        }
    }
}

/// Woher die Einstellungen kommen. `fromSuite: false`: nur die lokale Tabelle `settings`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct ConfigState {
    pub from_suite: bool,
    /// Letzte erfolgreiche Abholung aus der IUK-Suite.
    #[ts(type = "number | null")]
    pub fetched_at_ms: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct SuchlageMission {
    pub name: String,
    pub location: Option<String>,
    /// Aktiver Abschnitt.
    pub section: Option<String>,
    #[ts(type = "number | null")]
    pub started_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct SuchlageAircraft {
    pub gateway_sn: String,
    pub label: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub heading_deg: Option<f64>,
    /// Höhe über dem Startpunkt.
    pub elevation_m: Option<f64>,
    pub battery_percent: Option<u8>,
    pub airborne: bool,
}

/// Ein laufendes Livebild; abgespielt über `/api/suchlage/video/{gatewaySn}/…`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct SuchlageVideo {
    pub gateway_sn: String,
    pub label: Option<String>,
}

/// Der laufende Suchlage-Einsatz, schon nach den eingestellten Teilen gefiltert: Was ausgeschaltet
/// ist, kommt leer an. `mission: null` heißt: kein Einsatz läuft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct SuchlageView {
    pub mission: Option<SuchlageMission>,
    /// GeoJSON-FeatureCollection der gezeichneten Gebiete; `properties.color` als `#RRGGBB`.
    #[ts(type = "unknown")]
    pub areas: serde_json::Value,
    /// GeoJSON-FeatureCollection der Flugspuren; `properties.flightId`, `properties.running`.
    #[ts(type = "unknown")]
    pub tracks: serde_json::Value,
    pub aircraft: Vec<SuchlageAircraft>,
    pub video: Option<SuchlageVideo>,
}

/// Eine Meldung fürs Band aus der IUK-Suite, etwa „Nächste Lagebesprechung“. Jede läuft ab; der
/// Server gibt nur laufende heraus, die Oberfläche prüft den Ablauf mit ihrer Uhr noch einmal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct Notice {
    pub id: String,
    pub text: String,
    /// Bis wann heruntergezählt wird („in 42 min“); `null` ohne Countdown.
    #[ts(type = "number | null")]
    pub countdown_to_ms: Option<i64>,
    #[ts(type = "number")]
    pub expires_at_ms: i64,
}

/// QR-Code fürs Ehrenamt: wohin er führt und der Satz daneben. Nur `https://`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct Recruitment {
    pub url: String,
    pub text: String,
}

/// Welche Störung TomTom meldet. Abgefragt werden nur diese drei (`sources::tomtom`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub enum IncidentKind {
    RoadWorks,
    RoadClosed,
    LaneClosed,
}

/// Eine Baustelle oder Sperrung von TomTom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct TrafficIncident {
    pub id: String,
    pub kind: IncidentKind,
    pub description: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub road_numbers: Vec<String>,
    /// Voraussichtliches Ende, ISO 8601.
    pub end_time: Option<String>,
    /// GeoJSON-Geometrie (`Point` oder `LineString`).
    #[ts(type = "unknown")]
    pub geometry: serde_json::Value,
}

/// Ein fester Blitzer aus OpenStreetMap (`highway=speed_camera`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct SpeedCamera {
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    /// Wie in OSM, etwa `"50"`.
    pub maxspeed: Option<String>,
}

/// Verkehrslage. Ausgeschaltetes oder ohne TomTom-Schlüssel Unmögliches kommt als `false`/`null`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../frontend/src/contract/")]
pub struct TrafficView {
    /// Verkehrsfluss-Kacheln liegen unter `api/verkehr/fluss/{hell|dunkel}/{z}/{x}/{y}`.
    pub flow: bool,
    /// Der letzte Abruf einer Flusskachel scheiterte (Schlüssel, Kontingent, Netz).
    pub flow_error: Option<String>,
    pub incidents: Option<Sourced<Vec<TrafficIncident>>>,
    /// Nur auf der Online-Instanz, nie im FüKw (`BootConfig::online`).
    pub speed_cameras: Option<Sourced<Vec<SpeedCamera>>>,
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
    pub modules: Modules,
    pub config: ConfigState,
    /// `null`, wenn das Modul aus oder Suchlage nicht verbunden ist.
    pub suchlage: Option<Sourced<SuchlageView>>,
    /// Laufende Meldungen, die zuerst angelegte oben.
    pub notices: Vec<Notice>,
    /// `null`: kein QR-Code fürs Ehrenamt.
    pub recruitment: Option<Recruitment>,
    pub traffic: TrafficView,
}
