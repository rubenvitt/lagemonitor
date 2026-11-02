//! Feste Blitzer aus OpenStreetMap über Overpass (`highway=speed_camera`, Daten ODbL). Ändern sich
//! selten: einmal am Tag reicht (`poller::SPEED_CAMERA_INTERVAL`).

use std::time::Duration;

use reqwest::Url;
use serde::Deserialize;

use super::SourceError;
use crate::contract::{Location, SpeedCamera};
use crate::sources::tomtom::bbox;

pub const OVERPASS_BASE: &str = "https://overpass-api.de/api/interpreter";
/// Overpass bittet um eine Kennung; ohne kann es ablehnen.
const USER_AGENT: &str = "lagemonitor (DRK Uelzen, github.com/rubenvitt/lagemonitor)";
const TIMEOUT: Duration = Duration::from_secs(40);

pub fn query(loc: &Location) -> String {
    // Overpass will Süd, West, Nord, Ost.
    let (w, s, e, n) = bbox(loc);
    format!(
        r#"[out:json][timeout:30];node["highway"="speed_camera"]({s:.5},{w:.5},{n:.5},{e:.5});out body;"#
    )
}

#[derive(Deserialize)]
struct Body {
    elements: Vec<Raw>,
}

#[derive(Deserialize)]
struct Raw {
    id: i64,
    lat: f64,
    lon: f64,
    #[serde(default)]
    tags: std::collections::HashMap<String, String>,
}

pub fn parse(body: &[u8]) -> Result<Vec<SpeedCamera>, SourceError> {
    let raw: Body =
        serde_json::from_slice(body).map_err(|e| SourceError::Parse(format!("Overpass: {e}")))?;
    Ok(raw
        .elements
        .into_iter()
        .filter(|r| (-90.0..=90.0).contains(&r.lat) && (-180.0..=180.0).contains(&r.lon))
        .map(|mut r| SpeedCamera {
            id: format!("osm-node-{}", r.id),
            lat: r.lat,
            lon: r.lon,
            maxspeed: r.tags.remove("maxspeed"),
        })
        .collect())
}

pub async fn fetch(
    http: &reqwest::Client,
    base: &Url,
    loc: &Location,
) -> Result<Vec<SpeedCamera>, SourceError> {
    let res = http
        .post(base.clone())
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .form(&[("data", query(loc))])
        .timeout(TIMEOUT)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| SourceError::Http(format!("Overpass: {e}")))?;
    let body = res
        .bytes()
        .await
        .map_err(|e| SourceError::Http(format!("Overpass: {e}")))?;
    parse(&body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Form, Router, routing::post};
    use std::collections::HashMap;

    const FIXTURE: &[u8] = include_bytes!("../../tests/fixtures/overpass/speed_cameras.json");

    fn uelzen() -> Location {
        Location {
            lat: 52.9656,
            lon: 10.5594,
            label: "Uelzen".into(),
        }
    }

    #[test]
    fn parses_cameras_and_drops_bad_positions() {
        let list = parse(FIXTURE).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "osm-node-2465880131");
        assert_eq!(list[0].maxspeed.as_deref(), Some("50"));
        assert_eq!(list[1].maxspeed, None);
    }

    #[test]
    fn query_is_south_west_north_east() {
        let q = query(&uelzen());
        assert!(q.contains(r#"node["highway"="speed_camera"](52.52"#), "{q}");
    }

    #[tokio::test]
    async fn fetch_posts_the_query() {
        let app = Router::new().route(
            "/api/interpreter",
            post(|Form(f): Form<HashMap<String, String>>| async move {
                assert!(f["data"].contains("speed_camera"));
                FIXTURE
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = Url::parse(&format!(
            "http://{}/api/interpreter",
            listener.local_addr().unwrap()
        ))
        .unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let list = fetch(&reqwest::Client::new(), &base, &uelzen())
            .await
            .unwrap();
        assert_eq!(list.len(), 2);
    }
}
