// Übernommen aus Suchlage d621418 (27.10.2026): src/static_files.rs
//! Eingebettete SPA (rust-embed).

use axum::http::{StatusCode, Uri, header};
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
    (
        [
            (header::CONTENT_TYPE, mime.as_ref().to_string()),
            (header::CACHE_CONTROL, cache.to_string()),
        ],
        content.data,
    )
        .into_response()
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
    fn existing_file_is_served() {
        assert_eq!(
            resolve("/favicon.svg", |p| p == "favicon.svg"),
            Resolved::File("favicon.svg".into())
        );
    }
}
