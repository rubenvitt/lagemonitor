//! Konfiguration aus der IUK-Suite (Modul „Lagemonitor“, `GET /api/konfiguration`). Die Suite ist
//! die Verwaltung; der Monitor hält den zuletzt abgeholten Stand in `settings` und läuft damit auch
//! ohne Internet weiter.

use reqwest::Url;
use serde::Deserialize;

use super::{SourceError, endpoint, get_bytes_as};
use crate::config::SuiteLink;
use crate::contract::{Location, Modules, Region, Severity};
use crate::settings::{Settings, SuchlageLink};

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
