//! Externe Datenquellen.

use std::time::Duration;

use reqwest::Url;

pub mod nina;
pub mod weather;

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("Abruf fehlgeschlagen: {0}")]
    Http(String),
    #[error("Antwort nicht lesbar: {0}")]
    Parse(String),
}

const TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) async fn get_bytes(http: &reqwest::Client, url: Url) -> Result<Vec<u8>, SourceError> {
    let res = http
        .get(url.clone())
        .timeout(TIMEOUT)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| SourceError::Http(format!("{url}: {e}")))?;
    let bytes = res
        .bytes()
        .await
        .map_err(|e| SourceError::Http(format!("{url}: {e}")))?;
    Ok(bytes.to_vec())
}

pub(crate) fn endpoint(base: &Url, path: &str) -> Result<Url, SourceError> {
    // Ein Basispfad wie `/api31` bleibt nur mit abschließendem Slash erhalten.
    let mut base = base.clone();
    if !base.path().ends_with('/') {
        base.set_path(&format!("{}/", base.path()));
    }
    base.join(path)
        .map_err(|e| SourceError::Http(format!("URL {path}: {e}")))
}
