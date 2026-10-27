//! Livebild aus Suchlage über diesen Server: Der Kiosk-Browser kennt weder Adresse noch Token von
//! Suchlage. WHEP (WebRTC-Aushandlung) und HLS werden nur durchgereicht; das Bild selbst kommt bei
//! WebRTC direkt vom Videoserver, bei HLS über diesen Proxy.

use axum::body::{Body, Bytes};
use axum::extract::{Path, RawQuery, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::app::AppState;
use crate::settings::{self, SuchlageLink};
use crate::sources::{endpoint, suchlage};

const PASSED_HEADERS: [header::HeaderName; 3] = [
    header::CONTENT_TYPE,
    header::CONTENT_LENGTH,
    header::CONTENT_RANGE,
];

/// Seriennummer einer Fernsteuerung: nur Buchstaben und Ziffern.
pub fn valid_sn(sn: &str) -> bool {
    !sn.is_empty() && sn.len() <= 64 && sn.bytes().all(|b| b.is_ascii_alphanumeric())
}

/// Nur `[A-Za-z0-9._-]`-Segmente, nie `..` (wie Suchlage `media::proxy::safe_file`).
pub fn safe_file(file: &str) -> bool {
    !file.is_empty()
        && file.split('/').all(|seg| {
            !seg.is_empty()
                && seg != ".."
                && seg
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
}

fn plain(status: StatusCode, text: &'static str) -> Response {
    (status, text).into_response()
}

/// Nur mit eingeschaltetem Drohnenbild; sonst gibt es hier nichts abzuspielen.
async fn link(state: &AppState) -> Result<SuchlageLink, Box<Response>> {
    let cfg = settings::load(&state.pool)
        .await
        .map_err(|e| Box::new(e.into_response()))?;
    cfg.suchlage
        .filter(|l| cfg.modules.suchlage && l.video)
        .ok_or_else(|| Box::new(plain(StatusCode::NOT_FOUND, "Kein Livebild eingerichtet")))
}

fn passthrough(res: reqwest::Response) -> Response {
    let mut out = Response::builder()
        .status(res.status().as_u16())
        .header(header::CACHE_CONTROL, "no-store");
    for name in PASSED_HEADERS {
        if let Some(v) = res.headers().get(&name) {
            out = out.header(name, v.clone());
        }
    }
    out.body(Body::from_stream(res.bytes_stream()))
        .unwrap_or_else(|_| plain(StatusCode::BAD_GATEWAY, "Antwort nicht lesbar"))
}

fn unreachable(e: impl std::fmt::Display) -> Response {
    tracing::warn!(error = %e, "Suchlage-Video nicht erreichbar");
    plain(StatusCode::BAD_GATEWAY, "Suchlage nicht erreichbar")
}

pub async fn whep(State(state): State<AppState>, Path(sn): Path<String>, sdp: Bytes) -> Response {
    if !valid_sn(&sn) {
        return plain(StatusCode::NOT_FOUND, "Unbekannte Fernsteuerung");
    }
    let link = match link(&state).await {
        Ok(l) => l,
        Err(r) => return *r,
    };
    let url = match suchlage::base(&link).and_then(|b| endpoint(&b, &format!("video/{sn}/whep"))) {
        Ok(u) => u,
        Err(e) => return unreachable(e),
    };
    match state
        .http
        .post(url)
        .bearer_auth(&link.token)
        .header(header::CONTENT_TYPE, "application/sdp")
        .timeout(std::time::Duration::from_secs(10))
        .body(sdp)
        .send()
        .await
    {
        Ok(res) => passthrough(res),
        Err(e) => unreachable(e),
    }
}

pub async fn hls(
    State(state): State<AppState>,
    Path((sn, file)): Path<(String, String)>,
    RawQuery(query): RawQuery,
) -> Response {
    if !valid_sn(&sn) || !safe_file(&file) {
        return plain(StatusCode::NOT_FOUND, "Unbekannte Datei");
    }
    let link = match link(&state).await {
        Ok(l) => l,
        Err(r) => return *r,
    };
    let mut url =
        match suchlage::base(&link).and_then(|b| endpoint(&b, &format!("video/{sn}/hls/{file}"))) {
            Ok(u) => u,
            Err(e) => return unreachable(e),
        };
    url.set_query(query.as_deref());
    match state
        .http
        .get(url)
        .bearer_auth(&link.token)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
    {
        Ok(res) => passthrough(res),
        Err(e) => unreachable(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_checked_before_forwarding() {
        assert!(valid_sn("1581F5FHD23A"));
        assert!(!valid_sn("../x"));
        assert!(!valid_sn(""));
        assert!(safe_file("index.m3u8"));
        assert!(safe_file("seg/part_1.mp4"));
        assert!(!safe_file("../geheim"));
        assert!(!safe_file("a//b"));
    }
}
