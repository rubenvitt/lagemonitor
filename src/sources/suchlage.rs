//! Laufender Einsatz aus Suchlage (`GET /api/lagemonitor/v1/lage`, Großbild-Token als Bearer).
//! Gelesen wird nur, was der Monitor zeigt; Suchlage-Felder bleiben in diesem Modul.

use reqwest::Url;
use serde::Deserialize;
use serde_json::{Value, json};

use super::{SourceError, endpoint, get_bytes_as};
use crate::contract::{SuchlageAircraft, SuchlageMission, SuchlageVideo, SuchlageView};
use crate::settings::SuchlageLink;

/// Version der Suchlage-Schnittstelle (`/v1`).
pub const API_VERSION: u32 = 1;

#[derive(Deserialize)]
struct Lage {
    api_version: u32,
    mission: Option<Mission>,
    map: FeatureCollection,
    aircraft: Vec<Aircraft>,
    videos: Vec<Video>,
}

#[derive(Deserialize)]
struct Mission {
    name: String,
    location: Option<String>,
    started_at_ms: Option<i64>,
    section: Option<String>,
}

#[derive(Deserialize)]
struct FeatureCollection {
    features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
    geometry: Value,
    properties: Properties,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Properties {
    MapElement {
        name: String,
        color: String,
    },
    FlightTrack {
        flight_id: i64,
        ended_at_ms: Option<i64>,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct Point {
    lat: f64,
    lon: f64,
}

#[derive(Deserialize)]
struct Aircraft {
    gateway_sn: String,
    label: Option<String>,
    position: Point,
    heading_deg: Option<f64>,
    elevation_takeoff_m: Option<f64>,
    battery_percent: Option<u8>,
    airborne: bool,
}

#[derive(Deserialize)]
struct Video {
    gateway_sn: String,
    label: Option<String>,
}

fn collection(features: Vec<Value>) -> Value {
    json!({ "type": "FeatureCollection", "features": features })
}

/// Liest die Antwort und lässt weg, was ausgeschaltet ist. Ohne laufenden Einsatz ist alles leer.
pub fn parse(body: &[u8], link: &SuchlageLink) -> Result<SuchlageView, SourceError> {
    let lage: Lage =
        serde_json::from_slice(body).map_err(|e| SourceError::Parse(format!("Suchlage: {e}")))?;
    if lage.api_version != API_VERSION {
        return Err(SourceError::Parse(format!(
            "Suchlage-Schnittstelle in Version {}, erwartet {API_VERSION}",
            lage.api_version
        )));
    }
    let Some(mission) = lage.mission else {
        return Ok(SuchlageView {
            mission: None,
            areas: collection(Vec::new()),
            tracks: collection(Vec::new()),
            aircraft: Vec::new(),
            video: None,
        });
    };
    let (mut areas, mut tracks) = (Vec::new(), Vec::new());
    for f in lage.map.features {
        match f.properties {
            Properties::MapElement { name, color } if link.areas => areas.push(json!({
                "type": "Feature", "geometry": f.geometry,
                "properties": { "name": name, "color": color },
            })),
            Properties::FlightTrack {
                flight_id,
                ended_at_ms,
            } if link.tracks => tracks.push(json!({
                "type": "Feature", "geometry": f.geometry,
                "properties": { "flightId": flight_id, "running": ended_at_ms.is_none() },
            })),
            _ => {}
        }
    }
    Ok(SuchlageView {
        mission: Some(SuchlageMission {
            name: mission.name,
            location: mission.location,
            section: mission.section,
            started_at_ms: mission.started_at_ms,
        }),
        areas: collection(areas),
        tracks: collection(tracks),
        aircraft: lage
            .aircraft
            .into_iter()
            .map(|a| SuchlageAircraft {
                gateway_sn: a.gateway_sn,
                label: a.label,
                lat: a.position.lat,
                lon: a.position.lon,
                heading_deg: a.heading_deg,
                elevation_m: a.elevation_takeoff_m,
                battery_percent: a.battery_percent,
                airborne: a.airborne,
            })
            .collect(),
        video: link
            .video
            .then(|| lage.videos.into_iter().next())
            .flatten()
            .map(|v| SuchlageVideo {
                gateway_sn: v.gateway_sn,
                label: v.label,
            }),
    })
}

pub fn base(link: &SuchlageLink) -> Result<Url, SourceError> {
    let url: Url = link
        .url
        .parse()
        .map_err(|e| SourceError::Http(format!("Suchlage-Adresse: {e}")))?;
    endpoint(&url, "api/lagemonitor/v1/")
}

pub async fn fetch(
    http: &reqwest::Client,
    link: &SuchlageLink,
) -> Result<SuchlageView, SourceError> {
    let body = get_bytes_as(http, endpoint(&base(link)?, "lage")?, Some(&link.token)).await?;
    parse(&body, link)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Wie Suchlage `/api/lagemonitor/v1/lage` bei laufendem Einsatz (erfundene Werte).
    pub const RUNNING: &str = r##"{
      "api_version": 1,
      "server_time_ms": 1000,
      "mission": { "id": 3, "name": "Vermisste Person Oldenstadt", "location": "Oldenstadt", "started_at_ms": 500, "section": "Abschnitt Nord" },
      "map": { "type": "FeatureCollection", "features": [
        { "type": "Feature", "geometry": { "type": "Polygon", "coordinates": [[[10.55,52.97],[10.57,52.97],[10.57,52.98],[10.55,52.97]]] },
          "properties": { "kind": "map_element", "id": "e1", "name": "Suchgebiet A", "color": "#2D6CDF" } },
        { "type": "Feature", "geometry": { "type": "MultiLineString", "coordinates": [[[10.55,52.97],[10.56,52.975]]] },
          "properties": { "kind": "flight_track", "flight_id": 7, "name": "Flug 7", "color": "#8E44AD", "flugmappe_section_id": null, "started_at_ms": 600, "ended_at_ms": null } }
      ] },
      "aircraft": [ { "gateway_sn": "RC1", "label": "Drohne 1", "position": { "lat": 52.975, "lon": 10.56 }, "heading_deg": 90, "elevation_takeoff_m": 60, "battery_percent": 71, "airborne": true, "flight_id": 7 } ],
      "videos": [ { "gateway_sn": "RC1", "label": "Drohne 1" } ]
    }"##;

    pub fn link() -> SuchlageLink {
        SuchlageLink {
            url: "http://127.0.0.1:8090".into(),
            token: "geheim".into(),
            tracks: true,
            areas: true,
            video: true,
        }
    }

    fn count(v: &Value) -> usize {
        v["features"].as_array().unwrap().len()
    }

    #[test]
    fn running_mission_brings_everything() {
        let v = parse(RUNNING.as_bytes(), &link()).unwrap();
        assert_eq!(
            v.mission.unwrap().section.as_deref(),
            Some("Abschnitt Nord")
        );
        assert_eq!(v.areas["features"][0]["properties"]["color"], "#2D6CDF");
        assert_eq!(v.tracks["features"][0]["properties"]["flightId"], 7);
        assert_eq!(v.tracks["features"][0]["properties"]["running"], true);
        assert_eq!(v.aircraft[0].elevation_m, Some(60.0));
        assert_eq!(v.video.unwrap().gateway_sn, "RC1");
    }

    #[test]
    fn switched_off_parts_stay_empty() {
        let off = SuchlageLink {
            tracks: false,
            areas: false,
            video: false,
            ..link()
        };
        let v = parse(RUNNING.as_bytes(), &off).unwrap();
        assert!(v.mission.is_some());
        assert_eq!((count(&v.areas), count(&v.tracks)), (0, 0));
        assert!(v.video.is_none());
        assert_eq!(v.aircraft.len(), 1, "Drohnenpositionen gehören zum Modul");
    }

    #[test]
    fn without_mission_nothing_shows() {
        let body = r#"{"api_version":1,"server_time_ms":1,"mission":null,"map":{"type":"FeatureCollection","features":[]},"aircraft":[],"videos":[]}"#;
        let v = parse(body.as_bytes(), &link()).unwrap();
        assert!(v.mission.is_none());
        assert_eq!(count(&v.tracks), 0);
    }

    #[test]
    fn unknown_feature_kinds_are_skipped() {
        let body = RUNNING.replace("\"kind\": \"map_element\"", "\"kind\": \"zukunft\"");
        let v = parse(body.as_bytes(), &link()).unwrap();
        assert_eq!(count(&v.areas), 0);
        assert_eq!(count(&v.tracks), 1);
    }

    #[test]
    fn base_keeps_a_path() {
        let l = SuchlageLink {
            url: "https://example.org/suchlage".into(),
            ..link()
        };
        assert_eq!(
            base(&l).unwrap().as_str(),
            "https://example.org/suchlage/api/lagemonitor/v1/"
        );
    }
}
