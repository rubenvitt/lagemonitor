// Übernommen aus Suchlage d621418 (27.10.2026): src/static_files.rs
//! Eingebettete SPA (rust-embed).

use std::sync::LazyLock;

use axum::http::{HeaderName, HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};

#[derive(rust_embed::Embed)]
#[folder = "frontend/dist"]
struct Asset;

#[derive(Debug, PartialEq)]
pub enum Resolved {
    File(String),
    Index,
    NotFound,
}

/// Reine Entscheidung, damit sie ohne gebautes Frontend testbar ist.
pub fn resolve(path: &str, exists: impl Fn(&str) -> bool) -> Resolved {
    let path = path.trim_start_matches('/');
    if path.starts_with("api/") {
        return Resolved::NotFound;
    }
    if path.is_empty() {
        return Resolved::Index;
    }
    if exists(path) {
        return Resolved::File(path.to_string());
    }
    let has_extension = path
        .rsplit('/')
        .next()
        .is_some_and(|last| last.contains('.'));
    if has_extension {
        Resolved::NotFound
    } else {
        Resolved::Index
    }
}

/// Header, mit dem jede Antwort den Stand der eingebetteten Oberfläche nennt. Weicht er von dem Stand
/// ab, mit dem eine offene Seite geladen wurde, lädt sie sich neu (`frontend/src/stand.ts`).
pub const STAND_HEADER: HeaderName = HeaderName::from_static("x-lagemonitor-stand");

/// Der Stand ist der Hash der eingebetteten `index.html`: Sie nennt die gehashten Bundles, ändert sich
/// also mit jeder Änderung an der Oberfläche, aber nicht, wenn nur der Server neu gebaut wurde.
static STAND: LazyLock<Option<String>> =
    LazyLock::new(|| Asset::get("index.html").map(|f| hex::encode(&f.metadata.sha256_hash()[..8])));

/// `None`, solange das Frontend nicht gebaut ist; dann vergleicht die Seite nichts.
pub fn stand() -> Option<&'static str> {
    STAND.as_deref()
}

/// Hängt den Stand an jede Antwort.
pub async fn stand_header(mut res: Response) -> Response {
    if let Some(wert) = stand().and_then(|s| HeaderValue::from_str(s).ok()) {
        res.headers_mut().insert(STAND_HEADER, wert);
    }
    res
}

/// Schreibt den Stand als `<meta name="lagemonitor-stand">` in die Seite, damit sie weiß, womit sie
/// geladen wurde. Ohne `<head>` bleibt die Seite unverändert.
pub fn mit_stand(html: &str, stand: &str) -> String {
    match html.find("<head>") {
        Some(i) => {
            let ende = i + "<head>".len();
            format!(
                "{}<meta name=\"lagemonitor-stand\" content=\"{stand}\" />{}",
                &html[..ende],
                &html[ende..]
            )
        }
        None => html.to_string(),
    }
}

pub async fn serve(uri: Uri) -> Response {
    match resolve(uri.path(), |p| Asset::get(p).is_some()) {
        Resolved::File(path) => file(&path),
        Resolved::Index => file("index.html"),
        Resolved::NotFound => StatusCode::NOT_FOUND.into_response(),
    }
}

fn file(path: &str) -> Response {
    let Some(content) = Asset::get(path) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Frontend nicht gebaut: pnpm -C frontend build",
        )
            .into_response();
    };
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let headers = [
        (header::CONTENT_TYPE, mime.as_ref().to_string()),
        (header::CACHE_CONTROL, cache.to_string()),
    ];
    if path == "index.html"
        && let (Some(stand), Ok(html)) = (stand(), std::str::from_utf8(&content.data))
    {
        return (headers, mit_stand(html, stand)).into_response();
    }
    (headers, content.data).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spa_routes_fall_back_to_index() {
        assert_eq!(resolve("/lage/12", |_| false), Resolved::Index);
        assert_eq!(resolve("/", |_| false), Resolved::Index);
    }

    #[test]
    fn missing_asset_with_extension_is_404_not_index() {
        assert_eq!(resolve("/assets/app-123.js", |_| false), Resolved::NotFound);
    }

    #[test]
    fn api_paths_never_get_the_spa() {
        assert_eq!(resolve("/api/unknown", |_| true), Resolved::NotFound);
    }

    #[test]
    fn stand_lands_right_after_head() {
        assert_eq!(
            mit_stand("<html><head><title>x</title></head></html>", "ab12"),
            "<html><head><meta name=\"lagemonitor-stand\" content=\"ab12\" /><title>x</title></head></html>"
        );
    }

    #[test]
    fn page_without_head_stays_as_is() {
        assert_eq!(mit_stand("<p>x</p>", "ab12"), "<p>x</p>");
    }

    #[test]
    fn existing_file_is_served() {
        assert_eq!(
            resolve("/favicon.svg", |p| p == "favicon.svg"),
            Resolved::File("favicon.svg".into())
        );
    }
}
