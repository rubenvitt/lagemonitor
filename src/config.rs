//! Bootstrap-Konfiguration aus der Umgebung. Nur das, was vor der Datenbank feststehen muss –
//! alle fachlichen Einstellungen leben in der Tabelle `settings`.

use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct BootConfig {
    pub data_dir: PathBuf,
    pub binds: Vec<SocketAddr>,
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
                part.trim()
                    .parse()
                    .map_err(|e: std::net::AddrParseError| bind_error(e.to_string()))
            })
            .collect::<Result<Vec<SocketAddr>, _>>()?;
        Ok(Self { data_dir, binds })
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("lagemonitor.db")
    }

    pub fn tiles_path(&self) -> PathBuf {
        self.data_dir.join("karte.mbtiles")
    }
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
    fn invalid_bind_names_variable() {
        let err = BootConfig::from_lookup(lookup(&[("LAGEMONITOR_BIND", "nirgends")])).unwrap_err();
        assert_eq!(err.var, "LAGEMONITOR_BIND");
    }
}
