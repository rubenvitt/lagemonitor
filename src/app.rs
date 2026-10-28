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
    /// Letzter Abruffehler je Stand-Schlüssel (`poller::nina_key` …); fehlt der Eintrag, lief der letzte Abruf durch.
    pub errors: Arc<RwLock<HashMap<String, String>>>,
    /// Nur gesetzt, wenn `karte.mbtiles` beim Start vorhanden und lesbar war.
    pub tiles: Option<SqlitePool>,
    /// Letzter Suchlage-Stand. Nur im Speicher: Er altert in Sekunden, nach einem Neustart zählt er nicht.
    pub suchlage: Arc<RwLock<Option<SuchlageSnapshot>>>,
    /// Für Suite, Suchlage, den Video-Proxy und die Anmeldung.
    pub http: reqwest::Client,
    /// Endpunkte von Pocket ID, beim ersten Anmelden geholt.
    pub discovery: Arc<RwLock<Option<crate::auth::Discovery>>>,
    /// Zeitpunkt (ms) des letzten Abrufs, den ein unbekannter Link ausgelöst hat (`auth::link_login`).
    pub link_refresh_ms: Arc<tokio::sync::Mutex<i64>>,
    /// Letzter Versuch je Verkehrs-Stand (`poller::poll_traffic`), auch der gescheiterte: Ein Fehler
    /// darf das Kontingent nicht im Minutentakt aufbrauchen.
    pub traffic_attempts: Arc<tokio::sync::Mutex<HashMap<String, i64>>>,
    /// Holt TomTom-Kacheln nacheinander: Zehn Browser auf dieselbe Kachel kosten eine Abfrage.
    pub traffic_tile_fetch: Arc<tokio::sync::Mutex<()>>,
    /// Basisadresse von TomTom; Tests setzen einen lokalen Server ein.
    pub tomtom_base: reqwest::Url,
}

#[derive(Debug, Clone)]
pub struct SuchlageSnapshot {
    /// Adresse, von der der Stand kam; nach einem Wechsel gilt er nicht mehr.
    pub url: String,
    pub fetched_at_ms: i64,
    pub view: crate::contract::SuchlageView,
}

impl AppState {
    /// Eine kaputte Kartendatei darf den Monitor nicht aufhalten: Die Karte lädt dann online.
    pub async fn new(pool: SqlitePool, boot: Arc<BootConfig>) -> Self {
        // shortcut: Kartendatei wird nur beim Start geöffnet – nach dem Ablegen neu starten.
        let tiles_path = boot.tiles_path();
        let tiles = if tiles_path.exists() {
            open_tiles(&tiles_path)
                .await
                .inspect_err(|e| {
                    tracing::warn!(path = %tiles_path.display(), error = %e, "Kartendatei nicht lesbar, Karte lädt online");
                })
                .ok()
        } else {
            None
        };
        Self {
            pool,
            boot,
            errors: Arc::default(),
            tiles,
            suchlage: Arc::default(),
            http: reqwest::Client::new(),
            discovery: Arc::default(),
            link_refresh_ms: Arc::default(),
            traffic_attempts: Arc::default(),
            traffic_tile_fetch: Arc::default(),
            tomtom_base: crate::sources::tomtom::TOMTOM_BASE
                .parse()
                .expect("gültige Konstante"),
        }
    }
}

/// SQLite öffnet faul: Erst eine Abfrage zeigt, ob die Datei eine Kartendatei ist.
async fn open_tiles(path: &std::path::Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new().filename(path).read_only(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await?;
    sqlx::query("SELECT 1 FROM tiles LIMIT 1")
        .fetch_optional(&pool)
        .await?;
    Ok(pool)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/monitor", get(web::monitor::get))
        .route("/api/tiles/{z}/{x}/{y}", get(web::tiles::get))
        .route(
            "/api/verkehr/fluss/{theme}/{z}/{x}/{y}",
            get(web::traffic::flow_tile),
        )
        .route(
            "/api/suchlage/video/{gateway_sn}/whep",
            axum::routing::post(web::video::whep),
        )
        .route(
            "/api/suchlage/video/{gateway_sn}/hls/{*file}",
            get(web::video::hls),
        )
        .fallback(fallback)
        .layer(axum::middleware::map_response(static_files::stand_header))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Derselbe Monitor, aber nur nach Anmeldung über Pocket ID (`LAGEMONITOR_AUTH_BIND`).
pub fn router_with_login(state: AppState) -> Router {
    let guarded = router(state.clone()).layer(axum::middleware::from_fn_with_state(
        state.clone(),
        crate::auth::require,
    ));
    Router::new()
        .route("/auth/login", get(crate::auth::login))
        .route("/auth/callback", get(crate::auth::callback))
        .route("/auth/logout", get(crate::auth::logout))
        .route(
            "/auth/link",
            get(crate::auth::link_page).post(crate::auth::link_login),
        )
        .with_state(state)
        .merge(guarded)
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
        AppState::new(crate::db::test_pool().await, Arc::new(boot.unwrap())).await
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
    async fn traffic_layers_follow_settings_and_cameras_stay_off_in_the_vehicle() {
        let state = test_state(None).await;
        let json = json_of(
            router(state.clone())
                .oneshot(Request::get("/api/monitor").body(Body::empty()).unwrap())
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(json["traffic"]["flow"], false);
        assert!(json["traffic"]["incidents"].is_null());

        let mut cfg = crate::settings::load(&state.pool).await.unwrap();
        cfg.traffic = crate::settings::TrafficLink {
            tomtom_key: Some("schluessel0123456789".into()),
            flow: true,
            incidents: true,
            speed_cameras: true,
        };
        crate::settings::save(&state.pool, &cfg).await.unwrap();
        let json = json_of(
            router(state)
                .oneshot(Request::get("/api/monitor").body(Body::empty()).unwrap())
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(json["traffic"]["flow"], true);
        assert_eq!(
            json["traffic"]["incidents"]["stale"], true,
            "noch nie abgerufen"
        );
        assert!(
            json["traffic"]["speedCameras"].is_null(),
            "der Kiosk ist kein Online-Monitor"
        );
        assert!(
            !json.to_string().contains("schluessel0123456789"),
            "der Schlüssel bleibt auf dem Server"
        );
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
        let cfg = crate::settings::load(&state.pool).await.unwrap();
        let key = crate::poller::nina_key(&cfg.region);
        crate::store::save(&state.pool, &key, ten_min_ago, &Vec::<i32>::new())
            .await
            .unwrap();
        crate::poller::run_once(&state, &key, failing, || ten_min_ago + 1).await;
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
        crate::poller::run_once(&state, "nina", || async { Ok(vec![7]) }, || 1000).await;
        assert!(state.errors.read().await.is_empty());
        crate::poller::run_once(&state, "nina", failing, || 2000).await;
        assert_eq!(
            crate::store::load::<Vec<i32>>(&state.pool, "nina").await,
            Some((1000, vec![7]))
        );
        assert!(state.errors.read().await.contains_key("nina"));
        crate::poller::run_once(&state, "nina", || async { Ok(vec![8]) }, || 3000).await;
        assert!(state.errors.read().await.is_empty());
    }

    #[tokio::test]
    async fn panicking_fetch_is_recorded_and_next_fetch_lands() {
        let state = test_state(None).await;
        crate::poller::run_once(
            &state,
            "nina",
            || async { panic!("boom") as Result<Vec<i32>, crate::sources::SourceError> },
            || 1000,
        )
        .await;
        assert_eq!(state.errors.read().await["nina"], "Abruf abgestürzt");
        crate::poller::run_once(&state, "nina", || async { Ok(vec![5]) }, || 2000).await;
        assert!(state.errors.read().await.is_empty());
        assert_eq!(
            crate::store::load::<Vec<i32>>(&state.pool, "nina").await,
            Some((2000, vec![5]))
        );
    }

    #[tokio::test]
    async fn broken_cap_never_yields_fresh_snapshot_without_it() {
        let state = test_state(None).await;
        let dwd = crate::sources::nina::tests::dwd_id();
        let base = crate::sources::nina::tests::serve(move |f| f.starts_with(&dwd)).await;
        crate::poller::poll_nina(&state, &reqwest::Client::new(), &base).await;
        let res = router(state)
            .oneshot(Request::get("/api/monitor").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let json = json_of(res).await;
        assert!(json["warnings"]["data"].is_null(), "{json}");
        assert!(json["warnings"]["error"].is_string(), "{json}");
    }

    #[tokio::test]
    async fn region_change_hides_old_snapshot() {
        let state = test_state(None).await;
        let cfg = crate::settings::load(&state.pool).await.unwrap();
        let now = chrono::Utc::now().timestamp_millis();
        let key = crate::poller::nina_key(&cfg.region);
        crate::store::save(&state.pool, &key, now, &Vec::<i32>::new())
            .await
            .unwrap();
        sqlx::query("INSERT INTO settings (key, value) VALUES ('region', ?)")
            .bind(r#"{"ars":"031550000000","name":"Landkreis Celle"}"#)
            .execute(&state.pool)
            .await
            .unwrap();
        assert_ne!(cfg.region.ars, "031550000000");
        let res = router(state)
            .oneshot(Request::get("/api/monitor").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let json = json_of(res).await;
        assert_eq!(json["region"]["name"], "Landkreis Celle");
        assert!(json["warnings"]["data"].is_null(), "{json}");
    }

    #[tokio::test]
    async fn corrupt_tiles_file_falls_back_online() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("karte.mbtiles"), b"keine Datenbank").unwrap();
        let state = test_state(Some(dir.path())).await;
        let res = router(state)
            .oneshot(Request::get("/api/monitor").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(json_of(res).await["map"]["offline"], false);
    }

    #[tokio::test]
    async fn unreadable_setting_is_named_in_500() {
        let state = test_state(None).await;
        sqlx::query("INSERT INTO settings (key, value) VALUES ('warn_threshold', '\"Bananas\"')")
            .execute(&state.pool)
            .await
            .unwrap();
        let res = router(state)
            .oneshot(Request::get("/api/monitor").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            json_of(res).await["error"],
            "Einstellungen nicht lesbar: warn_threshold"
        );
    }

    #[tokio::test]
    async fn tiles_404_without_file() {
        let res = get_path("/api/tiles/1/0/0").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let res = get_path("/api/tiles/1/0/4294967295").await;
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

    /// Suite und Suchlage als ein Doppel: `/api/konfiguration` mit Token `suite`, `/api/lagemonitor/v1/lage`
    /// mit Token `geheim`. Die Suchlage-Adresse der Konfiguration zeigt auf dasselbe Doppel.
    async fn fake_suite_and_suchlage() -> reqwest::Url {
        use axum::http::HeaderMap;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let config = crate::sources::suite::tests::EXAMPLE
            .replace("http://127.0.0.1:8090", &base)
            .replace("\"video\": false", "\"video\": true");
        let auth = |h: &HeaderMap, want: &str| {
            h.get("authorization").and_then(|v| v.to_str().ok()) == Some(&format!("Bearer {want}"))
        };
        let app = Router::new()
            .route(
                "/api/konfiguration",
                get(move |h: HeaderMap| async move {
                    if auth(&h, "suite") {
                        (StatusCode::OK, config).into_response()
                    } else {
                        StatusCode::UNAUTHORIZED.into_response()
                    }
                }),
            )
            .route(
                "/api/lagemonitor/v1/lage",
                get(move |h: HeaderMap| async move {
                    if auth(&h, "geheim") {
                        (StatusCode::OK, crate::sources::suchlage::tests::RUNNING).into_response()
                    } else {
                        StatusCode::UNAUTHORIZED.into_response()
                    }
                }),
            );
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        base.parse().unwrap()
    }

    async fn suite_state(url: reqwest::Url, token: &str) -> AppState {
        let mut boot = BootConfig::from_lookup(|_| None).unwrap();
        boot.suite = Some(crate::config::SuiteLink {
            url,
            token: token.into(),
        });
        AppState::new(crate::db::test_pool().await, Arc::new(boot)).await
    }

    #[tokio::test]
    async fn config_from_the_suite_drives_the_monitor_and_suchlage() {
        let base = fake_suite_and_suchlage().await;
        let state = suite_state(base, "suite").await;
        crate::poller::poll_config(&state).await;
        crate::poller::poll_suchlage(&state).await;
        let res = router(state)
            .oneshot(Request::get("/api/monitor").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let json = json_of(res).await;
        assert_eq!(json["warnThreshold"], "Moderate");
        assert_eq!(json["modules"]["radar"], false);
        assert_eq!(json["config"]["fromSuite"], true);
        assert!(json["config"]["fetchedAtMs"].is_number(), "{json}");
        let suchlage = &json["suchlage"];
        assert_eq!(suchlage["stale"], false, "{json}");
        assert_eq!(
            suchlage["data"]["mission"]["name"],
            "Vermisste Person Oldenstadt"
        );
        assert_eq!(suchlage["data"]["video"]["gatewaySn"], "RC1");
        assert!(
            !json.to_string().contains("geheim"),
            "Der Token bleibt auf dem Server"
        );
    }

    #[tokio::test]
    async fn refused_suite_keeps_the_local_settings_and_says_so() {
        let base = fake_suite_and_suchlage().await;
        let state = suite_state(base, "falsch").await;
        crate::poller::poll_config(&state).await;
        let json = json_of(
            router(state)
                .oneshot(Request::get("/api/monitor").body(Body::empty()).unwrap())
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(json["warnThreshold"], "Severe");
        assert!(json["config"]["error"].is_string(), "{json}");
        assert!(
            json["suchlage"].is_null(),
            "Modul aus, solange nichts konfiguriert ist"
        );
    }

    #[tokio::test]
    async fn video_without_link_is_404() {
        let res = router(test_state(None).await)
            .oneshot(
                Request::post("/api/suchlage/video/RC1/whep")
                    .body(Body::from("v=0"))
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
