//! Konfiguration aus der IUK-Suite (Modul „Lagemonitor“, `GET /api/konfiguration`). Die Suite ist
//! die Verwaltung; der Monitor hält den zuletzt abgeholten Stand in `settings` und läuft damit auch
//! ohne Internet weiter.

use reqwest::Url;
use serde::Deserialize;

use super::{SourceError, endpoint, get_bytes_as};
use crate::config::SuiteLink;
use crate::contract::{Location, Modules, Notice, Recruitment, Region, Severity};
use crate::settings::{PublicLink, Settings, SuchlageLink, TrafficLink};

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
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawNotice {
    id: String,
    text: String,
    countdown_to: Option<String>,
    expires_at: String,
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
        id: raw.id,
        text: raw.text,
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
    })
}

pub fn config_url(link: &SuiteLink) -> Result<Url, SourceError> {
    endpoint(&link.url, "api/konfiguration")
}

pub async fn fetch(http: &reqwest::Client, link: &SuiteLink) -> Result<Settings, SourceError> {
    let body = get_bytes_as(http, config_url(link)?, Some(&link.token)).await?;
    parse(&body)
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
    fn a_suite_without_notices_still_works() {
        let s = parse(EXAMPLE.as_bytes()).unwrap();
        assert!(s.notices.is_empty());
        assert_eq!(s.recruitment, None);
    }

    #[test]
    fn notices_and_recruitment_are_read() {
        let body = EXAMPLE.replace(
            "\"suchlage\": {",
            r#""notices": [{ "id": "n1", "text": "Nächste Lagebesprechung", "countdownTo": "2026-10-27T13:30:00.000Z", "expiresAt": "2026-10-28T12:00:00.000Z" },
                           { "id": "n2", "text": "Funk Kanal 34", "countdownTo": null, "expiresAt": "2026-10-28T12:00:00.000Z" }],
               "recruitment": { "url": "https://drk.example.org/mitmachen", "text": "Lust mitzuhelfen?" },
               "suchlage": {"#,
        );
        let s = parse(body.as_bytes()).unwrap();
        assert_eq!(s.notices.len(), 2);
        assert_eq!(s.notices[0].countdown_to_ms, Some(1_793_107_800_000));
        assert_eq!(s.notices[0].expires_at_ms, 1_793_188_800_000);
        assert_eq!(s.notices[1].countdown_to_ms, None);
        assert_eq!(s.recruitment.unwrap().text, "Lust mitzuhelfen?");
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
}
