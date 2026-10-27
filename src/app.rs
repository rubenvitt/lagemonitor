//! Router und gemeinsamer Zustand.

use std::collections::HashMap;
use std::sync::Arc;

use axum::http::Uri;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::sync::RwLock;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::trace::TraceLayer;

use crate::config::BootConfig;
use crate::error::AppError;
use crate::static_files;
use crate::web;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub boot: Arc<BootConfig>,
    /// Letzter Abruffehler je Quelle; fehlt der Eintrag, lief der letzte Abruf durch.
    pub errors: Arc<RwLock<HashMap<&'static str, String>>>,
    /// Nur gesetzt, wenn `karte.mbtiles` beim Start vorhanden war.
    pub tiles: Option<SqlitePool>,
}

impl AppState {
    pub async fn new(pool: SqlitePool, boot: Arc<BootConfig>) -> Result<Self, AppError> {
        // shortcut: Kartendatei wird nur beim Start geöffnet – nach dem Ablegen neu starten.
        let tiles_path = boot.tiles_path();
        let tiles = if tiles_path.exists() {
            let options = SqliteConnectOptions::new()
                .filename(&tiles_path)
                .read_only(true);
            Some(
                SqlitePoolOptions::new()
                    .max_connections(2)
                    .connect_with(options)
                    .await?,
            )
        } else {
            None
        };
        Ok(Self {
            pool,
            boot,
            errors: Arc::default(),
            tiles,
        })
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/monitor", get(web::monitor::get))
        .route("/api/tiles/{z}/{x}/{y}", get(web::tiles::get))
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
        let state = test_state(None).await;
        router(state)
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    /// `data_dir` leer → Standardordner ohne Kartendatei.
    async fn test_state(data_dir: Option<&std::path::Path>) -> AppState {
        let boot = match data_dir {
            Some(dir) => {
                let dir = dir.to_str().unwrap().to_string();
                BootConfig::from_lookup(move |k| (k == "LAGEMONITOR_DATA_DIR").then(|| dir.clone()))
            }
            None => BootConfig::from_lookup(|_| None),
        };
        AppState::new(crate::db::test_pool().await, Arc::new(boot.unwrap()))
            .await
            .unwrap()
    }

    async fn json_of(res: Response) -> serde_json::Value {
        let body = res.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&body).unwrap()
    }

    async fn failing() -> Result<Vec<i32>, crate::sources::SourceError> {
        Err(crate::sources::SourceError::Http("kaputt".into()))
    }

    #[tokio::test]
    async fn empty_db_yields_null_data() {
        let res = get_path("/api/monitor").await;
        assert_eq!(res.status(), StatusCode::OK);
        let json = json_of(res).await;
        assert!(json["weather"]["data"].is_null());
        assert!(json["warnings"]["fetchedAtMs"].is_null());
        assert_eq!(json["warnings"]["stale"], true);
        assert_eq!(json["map"]["offline"], false);
    }

    #[tokio::test]
    async fn monitor_contains_settings() {
        let json = json_of(get_path("/api/monitor").await).await;
        assert_eq!(json["location"]["label"], "Uelzen");
        assert_eq!(json["warnThreshold"], "Severe");
    }

    #[tokio::test]
    async fn monitor_serves_snapshot_when_upstream_down() {
        let state = test_state(None).await;
        let ten_min_ago = chrono::Utc::now().timestamp_millis() - 600_000;
        crate::store::save(&state.pool, "nina", ten_min_ago, &Vec::<i32>::new())
            .await
            .unwrap();
        crate::poller::run_once(&state, "nina", failing, ten_min_ago + 1).await;
        let res = router(state)
            .oneshot(Request::get("/api/monitor").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let json = json_of(res).await;
        assert_eq!(json["warnings"]["data"], serde_json::json!([]));
        assert_eq!(json["warnings"]["stale"], true);
        assert!(json["warnings"]["error"].is_string());
    }

    #[tokio::test]
    async fn failed_fetch_keeps_previous_snapshot() {
        let state = test_state(None).await;
        crate::poller::run_once(&state, "nina", || async { Ok(vec![7]) }, 1000).await;
        assert!(state.errors.read().await.is_empty());
        crate::poller::run_once(&state, "nina", failing, 2000).await;
        assert_eq!(
            crate::store::load::<Vec<i32>>(&state.pool, "nina").await,
            Some((1000, vec![7]))
        );
        assert!(state.errors.read().await.contains_key("nina"));
        crate::poller::run_once(&state, "nina", || async { Ok(vec![8]) }, 3000).await;
        assert!(state.errors.read().await.is_empty());
    }

    #[tokio::test]
    async fn tiles_404_without_file() {
        let res = get_path("/api/tiles/1/0/0").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn tiles_flip_y() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("karte.mbtiles");
        let pool = SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .filename(&path)
                .create_if_missing(true),
        )
        .await
        .unwrap();
        sqlx::query(
            "CREATE TABLE tiles (zoom_level INTEGER, tile_column INTEGER, tile_row INTEGER, tile_data BLOB)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO tiles VALUES (1, 0, 1, x'1f8b0800')")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        let state = test_state(Some(dir.path())).await;
        let app = router(state);
        let res = app
            .clone()
            .oneshot(Request::get("/api/monitor").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(json_of(res).await["map"]["offline"], true);
        let res = app
            .clone()
            .oneshot(
                Request::get("/api/tiles/1/0/0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.headers()[header::CONTENT_ENCODING], "gzip");
        assert_eq!(
            res.headers()[header::CONTENT_TYPE],
            "application/x-protobuf"
        );
        let body = res.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], &[0x1f, 0x8b, 0x08, 0x00]);
        let res = app
            .oneshot(
                Request::get("/api/tiles/1/0/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
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
