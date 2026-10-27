//! Bootstrap-Konfiguration aus der Umgebung. Nur das, was vor der Datenbank feststehen muss –
//! alle fachlichen Einstellungen leben in der Tabelle `settings`.

use std::net::SocketAddr;
use std::path::PathBuf;

/// Abholung der Konfiguration aus der IUK-Suite (Modul „Lagemonitor“).
#[derive(Debug, Clone, PartialEq)]
pub struct SuiteLink {
    /// Wurzel des Moduls, z. B. `https://lagemonitor.iuk-ue.de`.
    pub url: reqwest::Url,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct BootConfig {
    pub data_dir: PathBuf,
    pub binds: Vec<SocketAddr>,
    /// `None`: keine Suite, es gilt allein die lokale Tabelle `settings`.
    pub suite: Option<SuiteLink>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
#[error("{var} ist ungültig: {reason}")]
pub struct BootError {
    pub var: &'static str,
    pub reason: String,
}

impl BootConfig {
    pub fn from_env() -> Result<Self, BootError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, BootError> {
        let data_dir =
            PathBuf::from(get("LAGEMONITOR_DATA_DIR").unwrap_or_else(|| "./data".into()));
        let bind_raw = get("LAGEMONITOR_BIND").unwrap_or_else(|| "127.0.0.1:8080".into());
        let bind_error = |reason: String| BootError {
            var: "LAGEMONITOR_BIND",
            reason,
        };
        let binds = bind_raw
            .split(',')
            .map(|part| {
                let addr: SocketAddr = part
                    .trim()
                    .parse()
                    .map_err(|e: std::net::AddrParseError| bind_error(e.to_string()))?;
                if addr.ip().is_unspecified() {
                    return Err(bind_error(format!(
                        "{addr} würde auf allen Schnittstellen lauschen – bitte konkrete Adressen angeben"
                    )));
                }
                Ok(addr)
            })
            .collect::<Result<Vec<SocketAddr>, _>>()?;
        let suite = suite_link(
            get("LAGEMONITOR_SUITE_URL").filter(|v| !v.trim().is_empty()),
            get("LAGEMONITOR_SUITE_TOKEN").filter(|v| !v.trim().is_empty()),
        )?;
        Ok(Self {
            data_dir,
            binds,
            suite,
        })
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("lagemonitor.db")
    }

    pub fn tiles_path(&self) -> PathBuf {
        self.data_dir.join("karte.mbtiles")
    }
}

/// Beides oder nichts: Eine Adresse ohne Token (oder umgekehrt) ist ein Tippfehler, kein Wunsch.
fn suite_link(url: Option<String>, token: Option<String>) -> Result<Option<SuiteLink>, BootError> {
    let (url, token) = match (url, token) {
        (None, None) => return Ok(None),
        (Some(url), Some(token)) => (url, token),
        (Some(_), None) => {
            return Err(BootError {
                var: "LAGEMONITOR_SUITE_TOKEN",
                reason: "fehlt, LAGEMONITOR_SUITE_URL ist gesetzt".into(),
            });
        }
        (None, Some(_)) => {
            return Err(BootError {
                var: "LAGEMONITOR_SUITE_URL",
                reason: "fehlt, LAGEMONITOR_SUITE_TOKEN ist gesetzt".into(),
            });
        }
    };
    let url: reqwest::Url = url.trim().parse().map_err(|e| BootError {
        var: "LAGEMONITOR_SUITE_URL",
        reason: format!("{e}"),
    })?;
    if !matches!(url.scheme(), "https" | "http") {
        return Err(BootError {
            var: "LAGEMONITOR_SUITE_URL",
            reason: "nur http oder https".into(),
        });
    }
    Ok(Some(SuiteLink {
        url,
        token: token.trim().to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn binds_default_to_loopback() {
        let cfg = BootConfig::from_lookup(lookup(&[])).unwrap();
        assert_eq!(cfg.binds, ["127.0.0.1:8080".parse().unwrap()]);
        assert_eq!(cfg.data_dir, PathBuf::from("./data"));
        assert_eq!(cfg.db_path(), PathBuf::from("./data/lagemonitor.db"));
        assert_eq!(cfg.tiles_path(), PathBuf::from("./data/karte.mbtiles"));
    }

    #[test]
    fn binds_accept_comma_list() {
        let cfg = BootConfig::from_lookup(lookup(&[(
            "LAGEMONITOR_BIND",
            "127.0.0.1:8080, 192.168.10.5:8080",
        )]))
        .unwrap();
        assert_eq!(cfg.binds.len(), 2);
        assert_eq!(cfg.binds[1].to_string(), "192.168.10.5:8080");
    }

    #[test]
    fn suite_needs_url_and_token() {
        assert_eq!(BootConfig::from_lookup(lookup(&[])).unwrap().suite, None);
        let cfg = BootConfig::from_lookup(lookup(&[
            ("LAGEMONITOR_SUITE_URL", "https://lagemonitor.iuk-ue.de"),
            ("LAGEMONITOR_SUITE_TOKEN", " abc "),
        ]))
        .unwrap();
        let suite = cfg.suite.unwrap();
        assert_eq!(suite.url.as_str(), "https://lagemonitor.iuk-ue.de/");
        assert_eq!(suite.token, "abc");
        let err = BootConfig::from_lookup(lookup(&[(
            "LAGEMONITOR_SUITE_URL",
            "https://lagemonitor.iuk-ue.de",
        )]))
        .unwrap_err();
        assert_eq!(err.var, "LAGEMONITOR_SUITE_TOKEN");
        let err = BootConfig::from_lookup(lookup(&[
            ("LAGEMONITOR_SUITE_URL", "ftp://x"),
            ("LAGEMONITOR_SUITE_TOKEN", "abc"),
        ]))
        .unwrap_err();
        assert_eq!(err.var, "LAGEMONITOR_SUITE_URL");
    }

    #[test]
    fn invalid_bind_names_variable() {
        let err = BootConfig::from_lookup(lookup(&[("LAGEMONITOR_BIND", "nirgends")])).unwrap_err();
        assert_eq!(err.var, "LAGEMONITOR_BIND");
    }

    #[test]
    fn unspecified_bind_is_rejected() {
        for bad in ["0.0.0.0:8080", "[::]:8080", "127.0.0.1:8080,0.0.0.0:8080"] {
            let err = BootConfig::from_lookup(lookup(&[("LAGEMONITOR_BIND", bad)])).unwrap_err();
            assert_eq!(err.var, "LAGEMONITOR_BIND", "{bad}");
        }
    }
}
