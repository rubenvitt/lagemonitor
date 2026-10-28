//! TomTom Traffic API: Baustellen und Sperrungen (Incident Details v5) und Kacheln des
//! Verkehrsflusses (Raster Flow Tiles v4). Der Schlüssel steht in keiner Fehlermeldung.
//!
//! Kontingent im Gratistarif: 2.500 Störungsabfragen und 200.000 Kacheln **im Monat**, geteilt von
//! FüKw und Online-Instanz (eine Suite, ein Schlüssel). Daher die langen Abstände in `poller` und
//! `web::traffic`.

use std::time::Duration;

use reqwest::Url;
use serde::Deserialize;

use super::SourceError;
use crate::contract::{IncidentKind, Location, TrafficIncident};

pub const TOMTOM_BASE: &str = "https://api.tomtom.com";
const TIMEOUT: Duration = Duration::from_secs(15);

/// Halbe Kantenlänge des Gebiets um den Standort. TomTom nimmt höchstens 10.000 km² je Abfrage;
/// 2 × 49,5 km im Quadrat sind 9.801 km².
const HALF_SIDE_KM: f64 = 49.5;

/// `minLon,minLat,maxLon,maxLat` um den Standort, auch für Overpass (`sources::osm`).
pub fn bbox(loc: &Location) -> (f64, f64, f64, f64) {
    let dlat = HALF_SIDE_KM / 111.32;
    let dlon = HALF_SIDE_KM / (111.32 * loc.lat.to_radians().cos().max(0.01));
    (
        loc.lon - dlon,
        loc.lat - dlat,
        loc.lon + dlon,
        loc.lat + dlat,
    )
}

/// 7 Fahrstreifen gesperrt, 8 Straße gesperrt, 9 Baustelle.
const CATEGORIES: &str = "7,8,9";
const FIELDS: &str = "{incidents{type,geometry{type,coordinates},properties{id,iconCategory,events{description},startTime,endTime,from,to,roadNumbers}}}";

pub fn incidents_url(base: &Url, key: &str, loc: &Location) -> Result<Url, SourceError> {
    let mut url = base
        .join("traffic/services/5/incidentDetails")
        .map_err(|e| SourceError::Http(format!("TomTom-Adresse: {e}")))?;
    let (w, s, e, n) = bbox(loc);
    url.query_pairs_mut()
        .append_pair("key", key)
        .append_pair("bbox", &format!("{w:.5},{s:.5},{e:.5},{n:.5}"))
        .append_pair("fields", FIELDS)
        .append_pair("language", "de-DE")
        .append_pair("categoryFilter", CATEGORIES)
        .append_pair("timeValidityFilter", "present");
    Ok(url)
}

/// `theme`: `dark` → `relative0-dark`, sonst `relative0` (die von TomTom empfohlenen Stile).
pub fn flow_tile_url(
    base: &Url,
    key: &str,
    dark: bool,
    z: u8,
    x: u32,
    y: u32,
) -> Result<Url, SourceError> {
    let style = if dark { "relative0-dark" } else { "relative0" };
    let mut url = base
        .join(&format!("traffic/map/4/tile/flow/{style}/{z}/{x}/{y}.png"))
        .map_err(|e| SourceError::Http(format!("TomTom-Adresse: {e}")))?;
    url.query_pairs_mut().append_pair("key", key);
    Ok(url)
}

/// GET ohne den Schlüssel in der Meldung: `reqwest` nennt sonst die ganze Adresse.
pub async fn get(http: &reqwest::Client, url: Url) -> Result<Vec<u8>, SourceError> {
    let shown = format!("{}{}", url.origin().ascii_serialization(), url.path());
    let res = http
        .get(url)
        .timeout(TIMEOUT)
        .send()
        .await
        .map_err(|e| SourceError::Http(format!("{shown}: {}", e.without_url())))?;
    let status = res.status();
    if !status.is_success() {
        // 403: Schlüssel ungültig oder Kontingent aufgebraucht, 429: zu viele Abfragen.
        return Err(SourceError::Http(format!(
            "{shown}: HTTP {}",
            status.as_u16()
        )));
    }
    let bytes = res
        .bytes()
        .await
        .map_err(|e| SourceError::Http(format!("{shown}: {}", e.without_url())))?;
    Ok(bytes.to_vec())
}

#[derive(Deserialize)]
struct Body {
    incidents: Vec<Raw>,
}

#[derive(Deserialize)]
struct Raw {
    geometry: serde_json::Value,
    properties: RawProps,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawProps {
    id: String,
    icon_category: u8,
    #[serde(default)]
    events: Vec<RawEvent>,
    end_time: Option<String>,
    from: Option<String>,
    to: Option<String>,
    #[serde(default)]
    road_numbers: Vec<String>,
}

#[derive(Deserialize)]
struct RawEvent {
    description: Option<String>,
}

fn kind(category: u8) -> Option<IncidentKind> {
    Some(match category {
        7 => IncidentKind::LaneClosed,
        8 => IncidentKind::RoadClosed,
        9 => IncidentKind::RoadWorks,
        _ => return None,
    })
}

pub fn parse_incidents(body: &[u8]) -> Result<Vec<TrafficIncident>, SourceError> {
    let raw: Body = serde_json::from_slice(body)
        .map_err(|e| SourceError::Parse(format!("TomTom-Störungen: {e}")))?;
    Ok(raw
        .incidents
        .into_iter()
        .filter_map(|r| {
            let p = r.properties;
            Some(TrafficIncident {
                kind: kind(p.icon_category)?,
                description: p.events.into_iter().find_map(|e| e.description),
                id: p.id,
                from: p.from,
                to: p.to,
                road_numbers: p.road_numbers,
                end_time: p.end_time,
                geometry: r.geometry,
            })
        })
        .collect())
}

pub async fn fetch_incidents(
    http: &reqwest::Client,
    base: &Url,
    key: &str,
    loc: &Location,
) -> Result<Vec<TrafficIncident>, SourceError> {
    parse_incidents(&get(http, incidents_url(base, key, loc)?).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, extract::Query, http::StatusCode, routing::get as route};
    use std::collections::HashMap;

    const FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/tomtom/incidents.json");

    fn uelzen() -> Location {
        Location {
            lat: 52.9656,
            lon: 10.5594,
            label: "Uelzen".into(),
        }
    }

    #[test]
    fn bbox_stays_below_the_tomtom_limit() {
        let (w, s, e, n) = bbox(&uelzen());
        let km_ns = (n - s) * 111.32;
        let km_ow = (e - w) * 111.32 * uelzen().lat.to_radians().cos();
        assert!(km_ns * km_ow < 10_000.0, "{}", km_ns * km_ow);
        assert!(km_ns * km_ow > 9_500.0);
    }

    #[test]
    fn parses_only_works_and_closures() {
        let list = parse_incidents(FIXTURE).unwrap();
        assert_eq!(list.len(), 2, "der Stau fällt weg");
        assert_eq!(list[0].kind, IncidentKind::RoadWorks);
        assert_eq!(list[0].description.as_deref(), Some("Bauarbeiten"));
        assert_eq!(list[0].road_numbers, vec!["B4"]);
        assert_eq!(list[0].geometry["type"], "LineString");
        assert_eq!(list[1].kind, IncidentKind::RoadClosed);
        assert_eq!(list[1].description, None);
    }

    #[test]
    fn broken_body_is_parse_error() {
        assert!(matches!(parse_incidents(b"{}"), Err(SourceError::Parse(_))));
    }

    #[test]
    fn tile_url_picks_the_style() {
        let base: Url = TOMTOM_BASE.parse().unwrap();
        let url = flow_tile_url(&base, "k", true, 10, 543, 330).unwrap();
        assert_eq!(
            url.as_str(),
            "https://api.tomtom.com/traffic/map/4/tile/flow/relative0-dark/10/543/330.png?key=k"
        );
        assert!(
            flow_tile_url(&base, "k", false, 1, 0, 0)
                .unwrap()
                .path()
                .contains("/relative0/")
        );
    }

    #[tokio::test]
    async fn fetch_sends_filter_and_hides_the_key_in_errors() {
        let app = Router::new().route(
            "/traffic/services/5/incidentDetails",
            route(|Query(q): Query<HashMap<String, String>>| async move {
                if q["key"] != "gut" {
                    return Err(StatusCode::FORBIDDEN);
                }
                assert_eq!(q["categoryFilter"], "7,8,9");
                assert_eq!(q["timeValidityFilter"], "present");
                assert!(q["bbox"].starts_with("9.821"));
                Ok(FIXTURE)
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let http = reqwest::Client::new();

        let list = fetch_incidents(&http, &base, "gut", &uelzen())
            .await
            .unwrap();
        assert_eq!(list.len(), 2);
        let err = fetch_incidents(&http, &base, "geheimer-schluessel", &uelzen())
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("HTTP 403"), "{err}");
        assert!(!err.contains("geheimer-schluessel"), "{err}");
    }
}
