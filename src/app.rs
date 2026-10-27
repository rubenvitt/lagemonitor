//! Router und gemeinsamer Zustand.

use std::sync::Arc;

use axum::http::Uri;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use sqlx::SqlitePool;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::trace::TraceLayer;

use crate::config::BootConfig;
use crate::error::AppError;
use crate::static_files;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub boot: Arc<BootConfig>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .fallback(fallback)
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

/// Unbekannte API-Pfade bekommen JSON, alles andere die SPA.
async fn fallback(uri: Uri) -> Response {
    if uri.path().starts_with("/api/") {
        AppError::NotFound.into_response()
    } else {
        static_files::serve(uri).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn get_path(path: &str) -> Response {
        let state = AppState {
            pool: crate::db::test_pool().await,
            boot: Arc::new(BootConfig::from_lookup(|_| None).unwrap()),
        };
        router(state)
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn health_ok() {
        let res = get_path("/api/health").await;
        assert_eq!(res.status(), StatusCode::OK);
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json, serde_json::json!({ "status": "ok" }));
    }

    #[tokio::test]
    async fn unknown_api_is_json_404() {
        let res = get_path("/api/nope").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let content_type = res.headers()[header::CONTENT_TYPE].to_str().unwrap();
        assert!(
            content_type.starts_with("application/json"),
            "{content_type}"
        );
    }
}
