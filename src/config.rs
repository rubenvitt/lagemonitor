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

/// Anmeldung über Pocket ID (OIDC) für die Listener aus `LAGEMONITOR_AUTH_BIND`.
#[derive(Clone, PartialEq)]
pub struct OidcConfig {
    /// z. B. `https://id.iuk-ue.de`; die Endpunkte kommen aus dessen Discovery-Dokument.
    pub issuer: reqwest::Url,
    pub client_id: String,
    pub client_secret: String,
    /// Adresse, unter der die Anmeldeseite den Monitor erreicht; daraus wird die Rücksprungadresse.
    pub public_url: reqwest::Url,
    /// Wer in mindestens einer dieser Gruppen ist, darf den Monitor sehen.
    pub groups: Vec<String>,
}

/// Ohne Geheimnis, damit es nie in ein Log gerät.
impl std::fmt::Debug for OidcConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OidcConfig")
            .field("issuer", &self.issuer.as_str())
            .field("client_id", &self.client_id)
            .field("public_url", &self.public_url.as_str())
            .field("groups", &self.groups)
            .finish_non_exhaustive()
    }
}

impl OidcConfig {
    pub fn redirect_uri(&self) -> String {
        format!(
            "{}/auth/callback",
            self.public_url.as_str().trim_end_matches('/')
        )
    }

    /// Pfad, unter dem der Monitor öffentlich hängt (`""` an der Wurzel, sonst etwa `/monitor`). Ein
    /// Proxy davor schneidet ihn ab; Weiterleitungen und Cookies brauchen ihn trotzdem.
    pub fn base_path(&self) -> &str {
        self.public_url.path().trim_end_matches('/')
    }

    /// Hinter https (Cloudflare, Traefik) bekommt das Sitzungs-Cookie `Secure`.
    pub fn secure_cookies(&self) -> bool {
        self.public_url.scheme() == "https"
    }
}

/// Vorgabe für `LAGEMONITOR_OIDC_GROUPS` nach dem Schema der Suite (`<modul>-nutzer`/`-admin`).
pub const DEFAULT_GROUPS: &str = "lagemonitor-nutzer,lagemonitor-admin";

#[derive(Debug, Clone)]
pub struct BootConfig {
    pub data_dir: PathBuf,
    /// Ohne Anmeldung: Kiosk (Loopback) und Fahrzeugnetz. Nie `0.0.0.0`.
    pub binds: Vec<SocketAddr>,
    /// Nur mit Anmeldung (Pocket ID): Zugriff von außen, etwa hinter Traefik in der Cloud.
    pub auth_binds: Vec<SocketAddr>,
    /// Gesetzt genau dann, wenn `auth_binds` nicht leer ist.
    pub oidc: Option<OidcConfig>,
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
        // Nicht gesetzt: Loopback wie bisher. Ausdrücklich leer: kein Listener ohne Anmeldung
        // (die Cloud-Instanz hat keinen Kiosk).
        let bind_raw = get("LAGEMONITOR_BIND").unwrap_or_else(|| "127.0.0.1:8080".into());
        let binds = parse_binds("LAGEMONITOR_BIND", &bind_raw)?;
        if let Some(addr) = binds.iter().find(|a| a.ip().is_unspecified()) {
            return Err(BootError {
                var: "LAGEMONITOR_BIND",
                reason: format!(
                    "{addr} würde auf allen Schnittstellen ohne Anmeldung lauschen – bitte konkrete Adressen angeben"
                ),
            });
        }
        // Hinter einem Proxy im Container ist `0.0.0.0` hier richtig: Jeder Zugriff braucht die Anmeldung.
        let auth_binds = parse_binds(
            "LAGEMONITOR_AUTH_BIND",
            &get("LAGEMONITOR_AUTH_BIND").unwrap_or_default(),
        )?;
        if binds.is_empty() && auth_binds.is_empty() {
            return Err(BootError {
                var: "LAGEMONITOR_BIND",
                reason:
                    "leer, und LAGEMONITOR_AUTH_BIND auch – der Monitor wäre nirgends erreichbar"
                        .into(),
            });
        }
        let oidc = if auth_binds.is_empty() {
            None
        } else {
            Some(oidc_config(&get)?)
        };
        let suite = suite_link(
            get("LAGEMONITOR_SUITE_URL").filter(|v| !v.trim().is_empty()),
            get("LAGEMONITOR_SUITE_TOKEN").filter(|v| !v.trim().is_empty()),
        )?;
        Ok(Self {
            data_dir,
            binds,
            auth_binds,
            oidc,
            suite,
        })
    }

    /// Die Online-Instanz: nur Listener mit Anmeldung, kein Kiosk. Nur sie zeigt Blitzer.
    pub fn online(&self) -> bool {
        self.binds.is_empty() && !self.auth_binds.is_empty()
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("lagemonitor.db")
    }

    pub fn tiles_path(&self) -> PathBuf {
        self.data_dir.join("karte.mbtiles")
    }
}

fn parse_binds(var: &'static str, raw: &str) -> Result<Vec<SocketAddr>, BootError> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse()
                .map_err(|e: std::net::AddrParseError| BootError {
                    var,
                    reason: format!("{part}: {e}"),
                })
        })
        .collect()
}

fn https_or_http(var: &'static str, raw: &str) -> Result<reqwest::Url, BootError> {
    let url: reqwest::Url = raw.trim().parse().map_err(|e| BootError {
        var,
        reason: format!("{e}"),
    })?;
    if !matches!(url.scheme(), "https" | "http") {
        return Err(BootError {
            var,
            reason: "nur http oder https".into(),
        });
    }
    Ok(url)
}

fn oidc_config(get: &impl Fn(&str) -> Option<String>) -> Result<OidcConfig, BootError> {
    let required = |var: &'static str| {
        get(var)
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .ok_or(BootError {
                var,
                reason: "fehlt, LAGEMONITOR_AUTH_BIND ist gesetzt".into(),
            })
    };
    let issuer = https_or_http(
        "LAGEMONITOR_OIDC_ISSUER",
        &required("LAGEMONITOR_OIDC_ISSUER")?,
    )?;
    let public_url = https_or_http(
        "LAGEMONITOR_PUBLIC_URL",
        &required("LAGEMONITOR_PUBLIC_URL")?,
    )?;
    let groups: Vec<String> = get("LAGEMONITOR_OIDC_GROUPS")
        .unwrap_or_else(|| DEFAULT_GROUPS.into())
        .split(',')
        .map(str::trim)
        .filter(|g| !g.is_empty())
        .map(String::from)
        .collect();
    if groups.is_empty() {
        return Err(BootError {
            var: "LAGEMONITOR_OIDC_GROUPS",
            reason: "leer – ohne Gruppe dürfte jedes Konto bei Pocket ID den Monitor sehen".into(),
        });
    }
    Ok(OidcConfig {
        issuer,
        client_id: required("LAGEMONITOR_OIDC_CLIENT_ID")?,
        client_secret: required("LAGEMONITOR_OIDC_CLIENT_SECRET")?,
        public_url,
        groups,
    })
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
    let url = https_or_http("LAGEMONITOR_SUITE_URL", &url)?;
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

    const OIDC: [(&str, &str); 5] = [
        ("LAGEMONITOR_AUTH_BIND", "0.0.0.0:8080"),
        ("LAGEMONITOR_OIDC_ISSUER", "https://id.iuk-ue.de"),
        ("LAGEMONITOR_OIDC_CLIENT_ID", "client"),
        ("LAGEMONITOR_OIDC_CLIENT_SECRET", "geheim"),
        ("LAGEMONITOR_PUBLIC_URL", "https://lagebild.iuk-ue.de/"),
    ];

    #[test]
    fn cloud_instance_listens_only_with_login() {
        let mut pairs = OIDC.to_vec();
        pairs.push(("LAGEMONITOR_BIND", ""));
        let cfg = BootConfig::from_lookup(lookup(&pairs)).unwrap();
        assert!(cfg.binds.is_empty());
        assert_eq!(cfg.auth_binds, ["0.0.0.0:8080".parse().unwrap()]);
        let oidc = cfg.oidc.unwrap();
        assert_eq!(oidc.groups, ["lagemonitor-nutzer", "lagemonitor-admin"]);
        assert_eq!(
            oidc.redirect_uri(),
            "https://lagebild.iuk-ue.de/auth/callback"
        );
        assert!(oidc.secure_cookies());
        assert_eq!(oidc.base_path(), "");
        assert!(
            !format!("{oidc:?}").contains("geheim"),
            "Geheimnis nie im Debug"
        );
    }

    #[test]
    fn login_needs_every_oidc_variable() {
        for (var, _) in &OIDC[1..] {
            let pairs: Vec<_> = OIDC.iter().filter(|(k, _)| k != var).copied().collect();
            let err = BootConfig::from_lookup(lookup(&pairs)).unwrap_err();
            assert_eq!(err.var, *var);
        }
        let mut pairs = OIDC.to_vec();
        pairs.push(("LAGEMONITOR_OIDC_GROUPS", " , "));
        let err = BootConfig::from_lookup(lookup(&pairs)).unwrap_err();
        assert_eq!(err.var, "LAGEMONITOR_OIDC_GROUPS");
    }

    #[test]
    fn without_auth_bind_there_is_no_login() {
        let cfg = BootConfig::from_lookup(lookup(&[])).unwrap();
        assert!(cfg.auth_binds.is_empty());
        assert!(cfg.oidc.is_none());
    }

    #[test]
    fn no_listener_at_all_is_rejected() {
        let err = BootConfig::from_lookup(lookup(&[("LAGEMONITOR_BIND", " ")])).unwrap_err();
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
