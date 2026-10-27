//! NINA-Warnungen: Dashboard je Regionalschlüssel, CAP-Details und GeoJSON je Warnung.

use std::time::Duration;

use reqwest::Url;
use serde::Deserialize;

use super::SourceError;
use crate::contract::{Provider, Severity, Warning};

const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
struct DashboardEntry {
    id: String,
    #[serde(default)]
    payload: DashboardPayload,
}

#[derive(Deserialize, Default)]
struct DashboardPayload {
    #[serde(default)]
    data: DashboardData,
}

#[derive(Deserialize, Default)]
struct DashboardData {
    provider: Option<String>,
}

#[derive(Deserialize)]
struct Cap {
    #[serde(default)]
    info: Vec<CapInfo>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct CapInfo {
    event: String,
    headline: String,
    description: String,
    instruction: Option<String>,
    severity: String,
    onset: Option<String>,
    expires: Option<String>,
    #[serde(rename = "senderName")]
    sender_name: Option<String>,
}

fn provider_from_name(name: &str) -> Option<Provider> {
    Some(match name.to_ascii_lowercase().as_str() {
        "dwd" => Provider::Dwd,
        "mowas" => Provider::Mowas,
        "katwarn" => Provider::Katwarn,
        "biwapp" => Provider::Biwapp,
        "lhp" => Provider::Lhp,
        "police" => Provider::Police,
        _ => return None,
    })
}

/// IDs der Warnungen samt Anbieter (falls das Dashboard ihn nennt).
pub fn parse_dashboard(body: &[u8]) -> Result<Vec<(String, Option<Provider>)>, SourceError> {
    let entries: Vec<DashboardEntry> =
        serde_json::from_slice(body).map_err(|e| SourceError::Parse(format!("Dashboard: {e}")))?;
    Ok(entries
        .into_iter()
        .map(|e| {
            let provider = e
                .payload
                .data
                .provider
                .as_deref()
                .and_then(provider_from_name);
            (e.id, provider)
        })
        .collect())
}

/// Ohne Anbieter im Dashboard entscheidet das ID-Präfix; unbekannt gilt als MoWaS.
pub fn provider_of(id: &str) -> Provider {
    match id.get(..3).map(str::to_ascii_lowercase).as_deref() {
        Some("dwd") => Provider::Dwd,
        Some("kat") => Provider::Katwarn,
        Some("biw") => Provider::Biwapp,
        Some("lhp") => Provider::Lhp,
        Some("pol") => Provider::Police,
        _ => Provider::Mowas,
    }
}

pub fn level_label(provider: Provider, severity: Severity) -> String {
    let label = match (provider, severity) {
        (_, Severity::Unknown) => "Unbekannt",
        (Provider::Dwd, Severity::Minor) => "Stufe 1",
        (Provider::Dwd, Severity::Moderate) => "Stufe 2",
        (Provider::Dwd, Severity::Severe) => "Stufe 3",
        (Provider::Dwd, Severity::Extreme) => "Stufe 4",
        (_, Severity::Minor) => "Gering",
        (_, Severity::Moderate) => "Mittel",
        (_, Severity::Severe) => "Hoch",
        (_, Severity::Extreme) => "Extrem",
    };
    label.to_string()
}

fn parse_severity(raw: &str) -> Severity {
    match raw {
        "Minor" => Severity::Minor,
        "Moderate" => Severity::Moderate,
        "Severe" => Severity::Severe,
        "Extreme" => Severity::Extreme,
        _ => Severity::Unknown,
    }
}

/// Fehlende oder kaputte Geometrie ist kein Fehler: die Warnung bleibt ohne Karte erhalten.
fn parse_geometry(geojson: Option<&[u8]>) -> Option<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_slice(geojson?).ok()?;
    let has_features = value
        .get("features")
        .and_then(|f| f.as_array())
        .is_some_and(|f| !f.is_empty());
    has_features.then_some(value)
}

pub fn parse_warning(
    id: &str,
    provider: Option<Provider>,
    cap: &[u8],
    geojson: Option<&[u8]>,
) -> Result<Warning, SourceError> {
    let cap: Cap = serde_json::from_slice(cap)
        .map_err(|e| SourceError::Parse(format!("Warnung {id}: {e}")))?;
    let info = cap
        .info
        .into_iter()
        .next()
        .ok_or_else(|| SourceError::Parse(format!("Warnung {id}: ohne info-Block")))?;
    let provider = provider.unwrap_or_else(|| provider_of(id));
    let severity = parse_severity(&info.severity);
    Ok(Warning {
        id: id.to_string(),
        provider,
        event: info.event,
        headline: info.headline,
        description: info.description,
        instruction: info.instruction,
        severity,
        level_label: level_label(provider, severity),
        onset: info.onset,
        expires: info.expires,
        sender_name: info.sender_name,
        geometry: parse_geometry(geojson),
    })
}

async fn get_bytes(http: &reqwest::Client, url: Url) -> Result<Vec<u8>, SourceError> {
    let res = http
        .get(url.clone())
        .timeout(TIMEOUT)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| SourceError::Http(format!("{url}: {e}")))?;
    let bytes = res
        .bytes()
        .await
        .map_err(|e| SourceError::Http(format!("{url}: {e}")))?;
    Ok(bytes.to_vec())
}

fn endpoint(base: &Url, path: &str) -> Result<Url, SourceError> {
    // Ein Basispfad wie `/api31` bleibt nur mit abschließendem Slash erhalten.
    let mut base = base.clone();
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }
    base.join(path)
        .map_err(|e| SourceError::Http(format!("URL {path}: {e}")))
}

pub async fn fetch(
    http: &reqwest::Client,
    base: &Url,
    ars: &str,
) -> Result<Vec<Warning>, SourceError> {
    let dashboard = get_bytes(http, endpoint(base, &format!("dashboard/{ars}.json"))?).await?;
    let mut warnings = Vec::new();
    for (id, provider) in parse_dashboard(&dashboard)? {
        // Eine unlesbare Warnung bricht den Abruf ab: lieber alter Stand als stille Lücke.
        let cap = get_bytes(http, endpoint(base, &format!("warnings/{id}.json"))?).await?;
        let geojson = match endpoint(base, &format!("warnings/{id}.geojson")) {
            Ok(url) => get_bytes(http, url).await.ok(),
            Err(_) => None,
        };
        warnings.push(parse_warning(&id, provider, &cap, geojson.as_deref())?);
    }
    Ok(warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, http::StatusCode, routing::get};

    const EMPTY: &[u8] = include_bytes!("../../tests/fixtures/nina/dashboard-leer.json");
    const FULL: &[u8] = include_bytes!("../../tests/fixtures/nina/dashboard-voll.json");
    const CAP_DWD: &[u8] = include_bytes!("../../tests/fixtures/nina/cap-dwd.json");
    const CAP_MOWAS: &[u8] = include_bytes!("../../tests/fixtures/nina/cap-mowas.json");
    const GEOJSON: &[u8] = include_bytes!("../../tests/fixtures/nina/geojson-dwd.json");

    fn dwd_id() -> String {
        parse_dashboard(FULL).unwrap().remove(0).0
    }

    #[test]
    fn empty_dashboard_yields_no_ids() {
        assert!(parse_dashboard(EMPTY).unwrap().is_empty());
    }

    #[test]
    fn dashboard_yields_ids() {
        let entries: Vec<serde_json::Value> = serde_json::from_slice(FULL).unwrap();
        let ids = parse_dashboard(FULL).unwrap();
        assert_eq!(ids.len(), entries.len());
        assert_eq!(ids[0].0, entries[0]["id"].as_str().unwrap());
        assert_eq!(ids[0].1, Some(Provider::Dwd));
        assert_eq!(ids[1].1, Some(Provider::Mowas));
    }

    #[test]
    fn parses_dwd_cap() {
        let id = dwd_id();
        let w = parse_warning(&id, Some(Provider::Dwd), CAP_DWD, Some(GEOJSON)).unwrap();
        assert_eq!(w.provider, Provider::Dwd);
        assert_eq!(w.event, "STURMBÖEN");
        assert_eq!(w.headline, "Amtliche WARNUNG vor STURMBÖEN");
        assert_eq!(w.severity, Severity::Moderate);
        assert_eq!(w.level_label, "Stufe 2");
        assert!(w.onset.is_some() && w.expires.is_some());
        assert!(w.geometry.is_some());
    }

    #[test]
    fn parses_mowas_cap_level_words() {
        let w = parse_warning("mow.x", Some(Provider::Mowas), CAP_MOWAS, None).unwrap();
        assert!(
            ["Gering", "Mittel", "Hoch", "Extrem", "Unbekannt"].contains(&w.level_label.as_str())
        );
        assert_eq!(w.provider, Provider::Mowas);
    }

    #[test]
    fn warning_without_geojson_is_kept() {
        let w = parse_warning(&dwd_id(), None, CAP_DWD, None).unwrap();
        assert!(w.geometry.is_none());
        // Provider fehlt im Dashboard: Ableitung aus dem ID-Präfix.
        assert_eq!(w.provider, Provider::Dwd);
    }

    #[test]
    fn unknown_severity_maps_to_unknown() {
        let mut cap: serde_json::Value = serde_json::from_slice(CAP_DWD).unwrap();
        cap["info"][0]["severity"] = "Bananas".into();
        let w = parse_warning("dwd.x", None, &serde_json::to_vec(&cap).unwrap(), None).unwrap();
        assert_eq!(w.severity, Severity::Unknown);
        assert_eq!(w.level_label, "Unbekannt");
    }

    #[test]
    fn provider_and_labels() {
        assert_eq!(provider_of("mow.DE-1"), Provider::Mowas);
        assert_eq!(provider_of("kat.1"), Provider::Katwarn);
        assert_eq!(provider_of("biw.1"), Provider::Biwapp);
        assert_eq!(provider_of("lhp.1"), Provider::Lhp);
        assert_eq!(provider_of("pol.1"), Provider::Police);
        assert_eq!(provider_of("zzz"), Provider::Mowas);
        assert_eq!(level_label(Provider::Dwd, Severity::Extreme), "Stufe 4");
        assert_eq!(level_label(Provider::Lhp, Severity::Severe), "Hoch");
    }

    #[tokio::test]
    async fn fetch_against_local_server() {
        let id = dwd_id();
        let app = Router::new()
            .route("/dashboard/150850000000.json", get(|| async { FULL }))
            .route(
                "/warnings/{file}",
                get(
                    |axum::extract::Path(file): axum::extract::Path<String>| async move {
                        if file.ends_with(".geojson") {
                            (StatusCode::NOT_FOUND, Vec::new())
                        } else if file.starts_with("dwd") {
                            (StatusCode::OK, CAP_DWD.to_vec())
                        } else {
                            (StatusCode::OK, CAP_MOWAS.to_vec())
                        }
                    },
                ),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let warnings = fetch(&reqwest::Client::new(), &base, "150850000000")
            .await
            .unwrap();
        assert_eq!(warnings.len(), 2);
        assert_eq!(warnings[0].id, id);
        assert!(warnings.iter().all(|w| w.geometry.is_none()));
    }
}
