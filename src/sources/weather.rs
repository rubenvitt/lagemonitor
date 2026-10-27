//! DWD-Wetter über Bright Sky (Task 3).

use chrono::{DateTime, Duration, Utc};
use reqwest::Url;
use serde::Deserialize;

use super::{SourceError, endpoint, get_bytes};
use crate::contract::{CurrentWeather, HourlyWeather, Location, Weather};

const HOURS_AHEAD: usize = 12;

#[derive(Deserialize)]
struct CurrentBody {
    weather: CurrentRaw,
}

// Bright Sky liefert fehlende Messwerte als `null`: alles außer dem Zeitstempel ist optional.
#[derive(Deserialize)]
struct CurrentRaw {
    timestamp: String,
    temperature: Option<f64>,
    condition: Option<String>,
    icon: Option<String>,
    wind_speed_10: Option<f64>,
    wind_gust_speed_10: Option<f64>,
    wind_direction_10: Option<f64>,
    precipitation_10: Option<f64>,
}

#[derive(Deserialize)]
struct HourlyBody {
    weather: Vec<HourlyRaw>,
}

#[derive(Deserialize)]
struct HourlyRaw {
    timestamp: String,
    temperature: Option<f64>,
    icon: Option<String>,
    precipitation: Option<f64>,
}

pub fn parse_current(body: &[u8]) -> Result<CurrentWeather, SourceError> {
    let raw = serde_json::from_slice::<CurrentBody>(body)
        .map_err(|e| SourceError::Parse(format!("Wetter aktuell: {e}")))?
        .weather;
    Ok(CurrentWeather {
        timestamp: raw.timestamp,
        temperature: raw.temperature,
        condition: raw.condition,
        icon: raw.icon,
        wind_speed: raw.wind_speed_10,
        wind_gust_speed: raw.wind_gust_speed_10,
        wind_direction: raw.wind_direction_10,
        precipitation: raw.precipitation_10,
    })
}

/// Nur Einträge nach `now`, höchstens die nächsten 12 Stunden.
pub fn parse_hourly(body: &[u8], now: DateTime<Utc>) -> Result<Vec<HourlyWeather>, SourceError> {
    let entries = serde_json::from_slice::<HourlyBody>(body)
        .map_err(|e| SourceError::Parse(format!("Wetter stündlich: {e}")))?
        .weather;
    let mut hourly = Vec::new();
    for e in entries {
        let ts = DateTime::parse_from_rfc3339(&e.timestamp)
            .map_err(|err| SourceError::Parse(format!("Zeitstempel {}: {err}", e.timestamp)))?;
        if ts > now {
            hourly.push(HourlyWeather {
                timestamp: e.timestamp,
                temperature: e.temperature,
                icon: e.icon,
                precipitation: e.precipitation,
            });
        }
    }
    hourly.truncate(HOURS_AHEAD);
    Ok(hourly)
}

pub async fn fetch(
    http: &reqwest::Client,
    base: &Url,
    loc: &Location,
    now: DateTime<Utc>,
) -> Result<Weather, SourceError> {
    let mut current_url = endpoint(base, "current_weather")?;
    current_url
        .query_pairs_mut()
        .append_pair("lat", &loc.lat.to_string())
        .append_pair("lon", &loc.lon.to_string());
    // 13 Stunden, damit nach dem Filter „nach jetzt“ noch 12 Einträge übrig bleiben.
    let mut hourly_url = endpoint(base, "weather")?;
    hourly_url
        .query_pairs_mut()
        .append_pair("lat", &loc.lat.to_string())
        .append_pair("lon", &loc.lon.to_string())
        .append_pair("date", &now.to_rfc3339())
        .append_pair("last_date", &(now + Duration::hours(13)).to_rfc3339());

    let current = parse_current(&get_bytes(http, current_url).await?)?;
    let hourly = parse_hourly(&get_bytes(http, hourly_url).await?, now)?;
    Ok(Weather { current, hourly })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::Location;
    use axum::{Router, extract::Query, routing::get};
    use chrono::{DateTime, Utc};
    use reqwest::Url;
    use std::collections::HashMap;

    const CURRENT: &[u8] = include_bytes!("../../tests/fixtures/brightsky/current.json");
    const HOURLY: &[u8] = include_bytes!("../../tests/fixtures/brightsky/hourly.json");

    fn at(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn parses_current() {
        let c = parse_current(CURRENT).unwrap();
        assert_eq!(c.timestamp, "2026-10-27T11:30:00+00:00");
        assert_eq!(c.temperature, Some(14.2));
        assert_eq!(c.condition.as_deref(), Some("dry"));
        assert_eq!(c.icon.as_deref(), Some("cloudy"));
        assert_eq!(c.wind_speed, Some(14.4));
        assert_eq!(c.wind_gust_speed, Some(14.0));
        assert_eq!(c.wind_direction, Some(190.0));
        assert_eq!(c.precipitation, Some(0.0));
    }

    #[test]
    fn hourly_starts_after_now_and_caps_at_12() {
        // Fixture: 13 Einträge von 12:00 bis 00:00 Uhr.
        let h = parse_hourly(HOURLY, at("2026-10-27T11:30:00Z")).unwrap();
        assert_eq!(h.len(), 12);
        assert_eq!(h[0].timestamp, "2026-10-27T12:00:00+00:00");
        // Ein Eintrag exakt zu „jetzt“ zählt nicht mehr.
        let h = parse_hourly(HOURLY, at("2026-10-27T12:00:00Z")).unwrap();
        assert_eq!(h[0].timestamp, "2026-10-27T13:00:00+00:00");
        assert_eq!(h.len(), 12);
        let h = parse_hourly(HOURLY, at("2026-10-27T23:00:00Z")).unwrap();
        assert_eq!(h.len(), 1);
    }

    #[test]
    fn missing_fields_are_none() {
        let body = br#"{"weather":{"timestamp":"2026-10-27T11:30:00+00:00","temperature":null,"icon":null,"wind_speed_10":null}}"#;
        let c = parse_current(body).unwrap();
        assert_eq!(c.temperature, None);
        assert_eq!(c.condition, None);
        assert_eq!(c.wind_gust_speed, None);
        let body = br#"{"weather":[{"timestamp":"2026-10-27T12:00:00+00:00","temperature":null,"icon":null,"precipitation":null}]}"#;
        let h = parse_hourly(body, at("2026-10-27T11:00:00Z")).unwrap();
        assert_eq!(
            (h[0].temperature, h[0].icon.clone(), h[0].precipitation),
            (None, None, None)
        );
    }

    #[test]
    fn broken_json_is_parse_error() {
        assert!(matches!(parse_current(b"nope"), Err(SourceError::Parse(_))));
        assert!(matches!(
            parse_hourly(b"{}", at("2026-10-27T11:00:00Z")),
            Err(SourceError::Parse(_))
        ));
    }

    #[tokio::test]
    async fn fetch_against_local_server() {
        let app = Router::new()
            .route("/current_weather", get(|| async { CURRENT }))
            .route(
                "/weather",
                get(|Query(q): Query<HashMap<String, String>>| async move {
                    assert_eq!(q["lat"], "52.9656");
                    assert_eq!(q["date"], "2026-10-27T11:30:00+00:00");
                    assert_eq!(q["last_date"], "2026-10-28T00:30:00+00:00");
                    HOURLY
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let loc = Location {
            lat: 52.9656,
            lon: 10.5594,
            label: "Uelzen".into(),
        };

        let w = fetch(
            &reqwest::Client::new(),
            &base,
            &loc,
            at("2026-10-27T11:30:00Z"),
        )
        .await
        .unwrap();
        assert_eq!(w.current.temperature, Some(14.2));
        assert_eq!(w.hourly.len(), 12);
    }

    #[tokio::test]
    async fn fetch_reports_http_error() {
        let app = Router::new();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let loc = Location {
            lat: 1.0,
            lon: 2.0,
            label: "x".into(),
        };
        let err = fetch(
            &reqwest::Client::new(),
            &base,
            &loc,
            at("2026-10-27T11:30:00Z"),
        )
        .await;
        assert!(matches!(err, Err(SourceError::Http(_))));
    }
}
