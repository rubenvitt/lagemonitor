//! Konfiguration aus der IUK-Suite (Modul „Lagemonitor“, `GET /api/konfiguration`). Die Suite ist
//! die Verwaltung; der Monitor hält den zuletzt abgeholten Stand in `settings` und läuft damit auch
//! ohne Internet weiter.

use reqwest::Url;
use serde::Deserialize;

use super::{SourceError, endpoint};
use crate::config::SuiteLink;
use crate::contract::{
    Location, Modules, Notice, NoticeCategory, NoticeColor, NoticeIcon, Position, Recruitment,
    Region, Severity, Vehicle,
};
use crate::settings::{
    MAP_ZOOM_DEFAULT, MAP_ZOOM_MAX, MAP_ZOOM_MIN, PublicLink, Settings, SuchlageLink, TrafficLink,
};

/// Version des Formats, die dieser Monitor versteht.
pub const CONFIG_VERSION: u32 = 1;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Body {
    version: u32,
    location: Location,
    region: Region,
    warn_threshold: Severity,
    modules: Modules,
    suchlage: Option<SuchlageLink>,
    /// Seit dem Meldungsband; eine Suite ohne das Feld schickt keine Meldungen.
    #[serde(default)]
    notices: Vec<RawNotice>,
    #[serde(default)]
    recruitment: Option<Recruitment>,
    /// Links ohne Anmeldung für die Online-Instanz; eine Suite ohne das Feld hat keine.
    #[serde(default)]
    public_links: Vec<PublicLink>,
    /// Verkehrslage; eine Suite ohne das Feld schaltet alles aus.
    #[serde(default)]
    traffic: Option<TrafficLink>,
    /// Kartenausschnitt; eine Suite ohne das Feld lässt die Vorgabe stehen.
    #[serde(default)]
    map: Option<RawMap>,
    /// Fahrzeuge auf der Karte (LGM-10); eine Suite ohne das Feld hat keine.
    #[serde(default)]
    vehicles: Vec<RawVehicle>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawVehicle {
    id: String,
    call_sign: String,
    symbol: String,
    lat: f64,
    lon: f64,
    at: String,
    #[serde(default)]
    own: bool,
}

/// Ein Zeichen aus kommplan hat gut 1 KB; viel mehr ist kein Zeichen, sondern ein Fehler.
const VEHICLE_SYMBOL_MAX: usize = 32 * 1024;

/// Ein Fahrzeug, das sich nicht zeichnen lässt, fällt still weg; der Rest der Konfiguration gilt.
fn vehicle(v: RawVehicle) -> Option<Vehicle> {
    let at_ms = chrono::DateTime::parse_from_rfc3339(&v.at)
        .ok()?
        .timestamp_millis();
    let ok = v.symbol.len() <= VEHICLE_SYMBOL_MAX
        && v.symbol.starts_with("<svg")
        && (-90.0..=90.0).contains(&v.lat)
        && (-180.0..=180.0).contains(&v.lon)
        && !v.call_sign.trim().is_empty();
    ok.then_some(Vehicle {
        id: v.id,
        call_sign: v.call_sign,
        symbol: v.symbol,
        lat: v.lat,
        lon: v.lon,
        at_ms,
        own: v.own,
    })
}

#[derive(Deserialize)]
struct RawMap {
    zoom: Option<f64>,
}

/// Eine Zoomstufe außerhalb des Bereichs gilt als der nächste Rand; ohne Zahl gilt die Vorgabe.
fn map_zoom(m: Option<RawMap>) -> f64 {
    m.and_then(|m| m.zoom)
        .filter(|z| z.is_finite())
        .map_or(MAP_ZOOM_DEFAULT, |z| z.clamp(MAP_ZOOM_MIN, MAP_ZOOM_MAX))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawNotice {
    id: String,
    text: String,
    countdown_to: Option<String>,
    expires_at: String,
    /// Seit den planbaren Meldungen; eine Suite ohne sie schickt nur, was schon läuft.
    #[serde(default)]
    show_from: Option<String>,
    /// Seit den Meldekategorien; eine Suite ohne sie schickt das Feld nicht.
    #[serde(default)]
    category: Option<RawCategory>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCategory {
    name: String,
    color: String,
    #[serde(default)]
    icon: Option<RawIcon>,
    #[serde(default)]
    alert_lead_minutes: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawIcon {
    view_box: String,
    paths: Vec<String>,
}

/// So lang wie in der Suite plus Luft; länger wird abgeschnitten, damit das Kürzel die Zeile nicht sprengt.
const CATEGORY_NAME_MAX: usize = 32;
/// Mehr Vorlauf als eine Stunde bietet die Suite nicht an.
const ALERT_LEAD_MAX: u32 = 60;
const ICON_PATHS_MAX: usize = 8;
const ICON_PATH_LEN_MAX: usize = 8_000;

/// Eine unbekannte Farbe (neuere Suite) gilt als Blau, wie eine Meldung ohne Kategorie.
fn color(c: &str) -> NoticeColor {
    match c {
        "teal" => NoticeColor::Teal,
        "green" => NoticeColor::Green,
        "ochre" => NoticeColor::Ochre,
        "orange" => NoticeColor::Orange,
        "purple" => NoticeColor::Purple,
        "grey" => NoticeColor::Grey,
        _ => NoticeColor::Blue,
    }
}

/// Ein Zeichen sind reine SVG-Pfaddaten; alles andere fällt still weg, der Name bleibt.
fn icon(i: RawIcon) -> Option<NoticeIcon> {
    let view_box_ok = !i.view_box.is_empty()
        && i.view_box.len() <= 32
        && i.view_box
            .bytes()
            .all(|b| b.is_ascii_digit() || b" .-".contains(&b));
    let path_ok = |p: &String| {
        !p.is_empty()
            && p.len() <= ICON_PATH_LEN_MAX
            && p.bytes().all(|b| {
                b.is_ascii_digit() || b" .,-eE".contains(&b) || b"MmLlHhVvCcSsQqTtAaZz".contains(&b)
            })
    };
    (view_box_ok
        && !i.paths.is_empty()
        && i.paths.len() <= ICON_PATHS_MAX
        && i.paths.iter().all(path_ok))
    .then_some(NoticeIcon {
        view_box: i.view_box,
        paths: i.paths,
    })
}

fn category(c: RawCategory) -> Option<NoticeCategory> {
    let name: String = c.name.trim().chars().take(CATEGORY_NAME_MAX).collect();
    if name.is_empty() {
        return None;
    }
    Some(NoticeCategory {
        name,
        color: color(&c.color),
        icon: c.icon.and_then(icon),
        alert_lead_minutes: c.alert_lead_minutes.map(|m| m.min(ALERT_LEAD_MAX)),
    })
}

fn ms(iso: &str) -> Result<i64, SourceError> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .map(|t| t.timestamp_millis())
        .map_err(|e| SourceError::Parse(format!("Meldung: Zeitpunkt {iso:?}: {e}")))
}

fn notice(raw: RawNotice) -> Result<Notice, SourceError> {
    Ok(Notice {
        countdown_to_ms: raw.countdown_to.as_deref().map(ms).transpose()?,
        expires_at_ms: ms(&raw.expires_at)?,
        show_from_ms: raw.show_from.as_deref().map(ms).transpose()?,
        id: raw.id,
        text: raw.text,
        category: raw.category.and_then(category),
    })
}

/// Ein QR-Code führt nur auf `https://`; alles andere fällt still weg, der Rest der Konfiguration gilt.
/// Höchstlänge wie in der Suite: so passt die Adresse sicher in einen QR-Code (Stufe M), und das
/// Kodieren im Browser kann nicht scheitern.
const RECRUITMENT_URL_MAX: usize = 300;

fn recruitment(r: Option<Recruitment>) -> Option<Recruitment> {
    r.filter(|r| {
        r.url.len() <= RECRUITMENT_URL_MAX
            && Url::parse(&r.url)
                .map(|u| u.scheme() == "https" && u.host_str().is_some())
                .unwrap_or(false)
    })
}

/// TomTom-Schlüssel sind kurze Folgen aus Buchstaben und Ziffern. Alles andere fällt still weg
/// (es landet in einer Adresse bei TomTom).
fn traffic(t: Option<TrafficLink>) -> TrafficLink {
    let mut t = t.unwrap_or_default();
    t.tomtom_key = t
        .tomtom_key
        .filter(|k| (16..=64).contains(&k.len()) && k.bytes().all(|b| b.is_ascii_alphanumeric()));
    t
}

pub fn parse(body: &[u8]) -> Result<Settings, SourceError> {
    let raw: Body = serde_json::from_slice(body)
        .map_err(|e| SourceError::Parse(format!("Konfiguration: {e}")))?;
    if raw.version != CONFIG_VERSION {
        return Err(SourceError::Parse(format!(
            "Konfiguration in Version {}, erwartet {CONFIG_VERSION}",
            raw.version
        )));
    }
    Ok(Settings {
        location: raw.location,
        region: raw.region,
        warn_threshold: raw.warn_threshold,
        modules: raw.modules,
        suchlage: raw.suchlage,
        notices: raw
            .notices
            .into_iter()
            .map(notice)
            .collect::<Result<_, _>>()?,
        recruitment: recruitment(raw.recruitment),
        // Nur ein SHA-256 in Hex ist ein Hash; alles andere fällt still weg.
        public_links: raw
            .public_links
            .into_iter()
            .filter(|l| l.hash.len() == 64 && l.hash.bytes().all(|b| b.is_ascii_hexdigit()))
            .map(|l| PublicLink {
                hash: l.hash.to_ascii_lowercase(),
                ..l
            })
            .collect(),
        traffic: traffic(raw.traffic),
        map_zoom: map_zoom(raw.map),
        vehicles: raw.vehicles.into_iter().filter_map(vehicle).collect(),
    })
}

pub fn config_url(link: &SuiteLink) -> Result<Url, SourceError> {
    endpoint(&link.url, "api/konfiguration")
}

/// Kopfzeile, in der ein Monitor mit GPS seinen Fix bei der Abholung mitschickt (keine eigene Anfrage).
pub const POSITION_HEADER: &str = "x-lagemonitor-position";

/// `<Breite>,<Länge>,<Alter in s>`; das Alter statt eines Zeitpunkts, damit die Uhr im Fahrzeug nicht zählt.
pub fn position_header(p: &Position, age_s: i64) -> String {
    format!("{:.6},{:.6},{}", p.lat, p.lon, age_s.max(0))
}

/// Antwort der Suite auf eine Abholung.
#[derive(Debug)]
pub enum Fetched {
    /// Neue Einstellungen samt ihrem `ETag` (fehlt bei einer Suite ohne).
    Changed(Box<Settings>, Option<String>),
    /// 304: seit dem mitgeschickten `ETag` unverändert.
    Unchanged,
}

/// Holt die Konfiguration; `position` ist der letzte Fix samt Alter, wenn dieser Monitor ein GPS hat.
/// Mit `etag` aus der letzten Abholung antwortet die Suite meist nur mit 304 ohne Körper: Sie wird
/// jede Minute gefragt, geändert wird selten. Die Position kommt auch dann an.
pub async fn fetch(
    http: &reqwest::Client,
    link: &SuiteLink,
    position: Option<String>,
    etag: Option<&str>,
) -> Result<Fetched, SourceError> {
    let url = config_url(link)?;
    let mut req = http.get(url.clone()).bearer_auth(&link.token);
    if let Some(p) = position {
        req = req.header(POSITION_HEADER, p);
    }
    if let Some(e) = etag {
        req = req.header(reqwest::header::IF_NONE_MATCH, e);
    }
    let res = req
        .timeout(super::TIMEOUT)
        .send()
        .await
        .map_err(|e| SourceError::Http(format!("{url}: {e}")))?;
    if res.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(Fetched::Unchanged);
    }
    let res = res
        .error_for_status()
        .map_err(|e| SourceError::Http(format!("{url}: {e}")))?;
    let new_etag = res
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let body = res
        .bytes()
        .await
        .map_err(|e| SourceError::Http(format!("{url}: {e}")))?;
    Ok(Fetched::Changed(Box::new(parse(&body)?), new_etag))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub const EXAMPLE: &str = r##"{
      "version": 1,
      "changedAt": "2026-10-27T18:00:00.000Z",
      "location": { "lat": 52.9656, "lon": 10.5594, "label": "Uelzen" },
      "region": { "ars": "033600000000", "name": "Landkreis Uelzen" },
      "warnThreshold": "Moderate",
      "modules": { "weather": true, "radar": false, "warningAreas": true, "warningList": true, "suchlage": true },
      "suchlage": { "url": "http://127.0.0.1:8090", "token": "geheim", "tracks": true, "areas": true, "video": false }
    }"##;

    #[test]
    fn example_from_the_suite_is_read() {
        let s = parse(EXAMPLE.as_bytes()).unwrap();
        assert_eq!(s.warn_threshold, Severity::Moderate);
        assert!(!s.modules.radar);
        assert_eq!(s.suchlage.unwrap().token, "geheim");
    }

    #[test]
    fn vehicles_are_read_and_unusable_ones_dropped() {
        assert!(parse(EXAMPLE.as_bytes()).unwrap().vehicles.is_empty());
        let svg = r#"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1 1\"></svg>"#;
        let big = format!("<svg>{}</svg>", "x".repeat(VEHICLE_SYMBOL_MAX));
        let v = |id: &str, symbol: &str, lat: f64, at: &str| {
            format!(
                r#"{{ "id": "{id}", "callSign": "Rotkreuz Uelzen 40-12-1", "symbol": "{symbol}", "lat": {lat}, "lon": 10.57, "at": "{at}", "own": true }}"#
            )
        };
        let list = [
            v("gut", svg, 52.96, "2026-11-02T14:00:00.000Z"),
            v(
                "kein-svg",
                "<script>alert(1)</script>",
                52.96,
                "2026-11-02T14:00:00.000Z",
            ),
            v("zu-gross", &big, 52.96, "2026-11-02T14:00:00.000Z"),
            v("breite", svg, 91.0, "2026-11-02T14:00:00.000Z"),
            v("zeit", svg, 52.96, "gestern"),
        ]
        .join(",");
        let body = EXAMPLE.replace(
            "\"suchlage\": {",
            &format!(r#""vehicles": [{list}], "suchlage": {{"#),
        );
        let s = parse(body.as_bytes()).unwrap();
        assert_eq!(s.vehicles.len(), 1);
        assert_eq!(s.vehicles[0].id, "gut");
        assert_eq!(s.vehicles[0].at_ms, 1_793_628_000_000);
        assert!(s.vehicles[0].own);
    }

    #[test]
    fn the_position_header_carries_the_age_not_the_clock() {
        let p = Position {
            lat: 52.963712345,
            lon: 10.5772,
        };
        assert_eq!(position_header(&p, 12), "52.963712,10.577200,12");
        assert_eq!(position_header(&p, -3), "52.963712,10.577200,0");
    }

    #[test]
    fn a_suite_without_notices_still_works() {
        let s = parse(EXAMPLE.as_bytes()).unwrap();
        assert!(s.notices.is_empty());
        assert_eq!(s.recruitment, None);
    }

    #[test]
    fn notices_and_recruitment_are_read() {
        let body = EXAMPLE.replace(
            "\"suchlage\": {",
            r#""notices": [{ "id": "n1", "text": "Nächste Lagebesprechung", "countdownTo": "2026-10-27T13:30:00.000Z", "expiresAt": "2026-10-28T12:00:00.000Z", "showFrom": "2026-10-27T13:00:00.000Z" },
                           { "id": "n2", "text": "Funk Kanal 34", "countdownTo": null, "expiresAt": "2026-10-28T12:00:00.000Z" }],
               "recruitment": { "url": "https://drk.example.org/mitmachen", "text": "Lust mitzuhelfen?" },
               "suchlage": {"#,
        );
        let s = parse(body.as_bytes()).unwrap();
        assert_eq!(s.notices.len(), 2);
        assert_eq!(s.notices[0].countdown_to_ms, Some(1_793_107_800_000));
        assert_eq!(s.notices[0].expires_at_ms, 1_793_188_800_000);
        assert_eq!(s.notices[0].show_from_ms, Some(1_793_106_000_000));
        assert_eq!(s.notices[1].countdown_to_ms, None);
        assert_eq!(s.notices[1].show_from_ms, None);
        assert_eq!(s.recruitment.unwrap().text, "Lust mitzuhelfen?");
    }

    #[test]
    fn categories_are_read_and_bad_parts_dropped() {
        let body = EXAMPLE.replace(
            "\"suchlage\": {",
            r#""notices": [
                 { "id": "n1", "text": "Lage", "countdownTo": null, "expiresAt": "2026-10-28T12:00:00.000Z",
                   "category": { "name": "Besprechung", "color": "purple", "icon": { "viewBox": "0 0 48 48", "paths": ["M14 4A4 4 0 0 0 14 12z"] }, "alertLeadMinutes": 10 } },
                 { "id": "n2", "text": "Neu", "countdownTo": null, "expiresAt": "2026-10-28T12:00:00.000Z",
                   "category": { "name": "  Funk  ", "color": "magenta", "icon": { "viewBox": "0 0 48 48", "paths": ["<script>"] }, "alertLeadMinutes": 600 } },
                 { "id": "n3", "text": "Leer", "countdownTo": null, "expiresAt": "2026-10-28T12:00:00.000Z",
                   "category": { "name": " ", "color": "blue", "icon": null, "alertLeadMinutes": null } }],
               "suchlage": {"#,
        );
        let s = parse(body.as_bytes()).unwrap();
        let c = s.notices[0].category.as_ref().unwrap();
        assert_eq!(c.name, "Besprechung");
        assert_eq!(c.color, NoticeColor::Purple);
        assert_eq!(
            c.icon.as_ref().unwrap().paths,
            vec!["M14 4A4 4 0 0 0 14 12z".to_string()]
        );
        assert_eq!(c.alert_lead_minutes, Some(10));
        let c = s.notices[1].category.as_ref().unwrap();
        assert_eq!(c.name, "Funk");
        assert_eq!(c.color, NoticeColor::Blue);
        assert_eq!(c.icon, None);
        assert_eq!(c.alert_lead_minutes, Some(60));
        assert_eq!(s.notices[2].category, None);
    }

    #[test]
    fn public_links_are_read_and_bad_hashes_dropped() {
        let good = "AB".repeat(32);
        let body = EXAMPLE.replace(
            "\"suchlage\": {",
            &format!(
                r#""publicLinks": [{{ "id": "l1", "label": "Rechner Wache", "hash": "{good}" }},
                                  {{ "id": "l2", "label": "kaputt", "hash": "geheim" }}],
                   "suchlage": {{"#
            ),
        );
        let s = parse(body.as_bytes()).unwrap();
        assert_eq!(s.public_links.len(), 1);
        assert_eq!(s.public_links[0].label, "Rechner Wache");
        assert_eq!(s.public_links[0].hash, "ab".repeat(32));
        assert!(parse(EXAMPLE.as_bytes()).unwrap().public_links.is_empty());
    }

    #[test]
    fn traffic_is_read_and_a_bad_key_dropped() {
        assert_eq!(
            parse(EXAMPLE.as_bytes()).unwrap().traffic,
            TrafficLink::default()
        );
        let with = |key: &str| {
            EXAMPLE.replace(
                "\"suchlage\": {",
                &format!(
                    r#""traffic": {{ "tomtomKey": "{key}", "flow": true, "incidents": true, "speedCameras": false }}, "suchlage": {{"#
                ),
            )
        };
        let t = parse(with("AbCdEf0123456789AbCdEf0123456789").as_bytes())
            .unwrap()
            .traffic;
        assert_eq!(
            t.tomtom_key.as_deref(),
            Some("AbCdEf0123456789AbCdEf0123456789")
        );
        assert!(t.flow && t.incidents && !t.speed_cameras);
        let t = parse(with("abc&key=x0123456789abcdef").as_bytes())
            .unwrap()
            .traffic;
        assert_eq!(t.tomtom_key, None);
        assert!(t.flow, "die Schalter gelten weiter");
    }

    #[test]
    fn map_zoom_is_read_and_clamped() {
        assert_eq!(
            parse(EXAMPLE.as_bytes()).unwrap().map_zoom,
            MAP_ZOOM_DEFAULT
        );
        let with = |map: &str| {
            let body = EXAMPLE.replace(
                "\"suchlage\": {",
                &format!(r#""map": {map}, "suchlage": {{"#),
            );
            parse(body.as_bytes()).unwrap().map_zoom
        };
        assert_eq!(with(r#"{ "zoom": 11.5 }"#), 11.5);
        assert_eq!(with(r#"{ "zoom": 30 }"#), MAP_ZOOM_MAX);
        assert_eq!(with(r#"{ "zoom": 1 }"#), MAP_ZOOM_MIN);
        assert_eq!(with(r#"{ "zoom": null }"#), MAP_ZOOM_DEFAULT);
        assert_eq!(with("{}"), MAP_ZOOM_DEFAULT);
    }

    #[test]
    fn recruitment_needs_https() {
        let body = EXAMPLE.replace(
            "\"suchlage\": {",
            r#""recruitment": { "url": "javascript:alert(1)", "text": "x" }, "suchlage": {"#,
        );
        assert_eq!(parse(body.as_bytes()).unwrap().recruitment, None);
    }

    #[test]
    fn recruitment_url_has_a_length_limit() {
        let lang = format!("https://example.org/{}", "a".repeat(300));
        let body = EXAMPLE.replace(
            "\"suchlage\": {",
            &format!(r#""recruitment": {{ "url": "{lang}", "text": "x" }}, "suchlage": {{"#),
        );
        assert_eq!(parse(body.as_bytes()).unwrap().recruitment, None);
    }

    #[test]
    fn other_versions_are_refused() {
        let body = EXAMPLE.replace("\"version\": 1", "\"version\": 2");
        assert!(matches!(parse(body.as_bytes()), Err(SourceError::Parse(_))));
    }

    #[test]
    fn url_keeps_a_base_path() {
        let link = SuiteLink {
            url: "https://iuk-ue.de/m/lagemonitor".parse().unwrap(),
            token: "t".into(),
        };
        assert_eq!(
            config_url(&link).unwrap().as_str(),
            "https://iuk-ue.de/m/lagemonitor/api/konfiguration"
        );
    }

    /// Suite-Attrappe mit `ETag "v1"`: 304 bei passendem `If-None-Match`, sonst `EXAMPLE`.
    async fn etag_suite() -> SuiteLink {
        use axum::http::{HeaderMap, StatusCode, header};
        use axum::response::IntoResponse;
        let app = axum::Router::new().route(
            "/api/konfiguration",
            axum::routing::get(|h: HeaderMap| async move {
                if h.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some("\"v1\"") {
                    return StatusCode::NOT_MODIFIED.into_response();
                }
                ([(header::ETAG, "\"v1\"")], EXAMPLE).into_response()
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        SuiteLink {
            url: url.parse().unwrap(),
            token: "t".into(),
        }
    }

    #[tokio::test]
    async fn unchanged_settings_come_back_as_304() {
        let link = etag_suite().await;
        let http = crate::http::client();
        match fetch(&http, &link, None, None).await.unwrap() {
            Fetched::Changed(s, etag) => {
                assert_eq!(s.warn_threshold, Severity::Moderate);
                assert_eq!(etag.as_deref(), Some("\"v1\""));
            }
            Fetched::Unchanged => panic!("ohne ETag kommt alles"),
        }
        assert!(matches!(
            fetch(&http, &link, None, Some("\"v1\"")).await.unwrap(),
            Fetched::Unchanged
        ));
        assert!(matches!(
            fetch(&http, &link, None, Some("\"alt\"")).await.unwrap(),
            Fetched::Changed(..)
        ));
    }
}
