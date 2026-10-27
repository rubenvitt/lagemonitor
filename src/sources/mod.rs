//! Externe Datenquellen.

use std::time::Duration;

use reqwest::Url;

pub mod nina;
pub mod suchlage;
pub mod suite;
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
    get_bytes_as(http, url, None).await
}

/// Wie `get_bytes`, mit `Authorization: Bearer …`. Der Token steht nie in einer Fehlermeldung.
pub(crate) async fn get_bytes_as(
    http: &reqwest::Client,
    url: Url,
    bearer: Option<&str>,
) -> Result<Vec<u8>, SourceError> {
    let mut req = http.get(url.clone());
    if let Some(token) = bearer {
        req = req.bearer_auth(token);
    }
    let res = req
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
