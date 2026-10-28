//! `GET /api/verkehr/fluss/{hell|dunkel}/{z}/{x}/{y}`: Kacheln des Verkehrsflusses von TomTom,
//! durchgereicht und zwischengespeichert. Der Browser sieht den Schlüssel nie.
//!
//! Kontingent (`sources::tomtom`): 200.000 Kacheln im Monat für FüKw und Online-Instanz zusammen.
//! Jede Kachel kommt höchstens alle `FLOW_TTL_MS` von TomTom, und je Instanz und Tag höchstens
//! `TILES_PER_DAY` (2 × 3.000 × 31 = 186.000). Ist die Grenze erreicht oder TomTom nicht
//! erreichbar, gilt die letzte Kachel weiter.

use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::app::AppState;
use crate::error::AppError;
use crate::settings;
use crate::sources::tomtom;

pub const FLOW_TTL_MS: i64 = 30 * 60_000;
pub const TILES_PER_DAY: i64 = 3_000;
/// Stand-Schlüssel für Fehler beim Abruf (`AppState::errors`).
pub const FLOW_KEY: &str = "tomtom-fluss";
/// Suchlage zoomt bis 15; tiefer lohnt kein Verkehrsfluss.
const MAX_ZOOM: u8 = 18;

fn png(body: Vec<u8>) -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            // Die Oberfläche hängt den Stand an die Adresse (`trafficTiles`); so lange gilt die Kachel.
            (header::CACHE_CONTROL, "public, max-age=1800"),
        ],
        body,
    )
        .into_response()
}

async fn cached(pool: &SqlitePool, dark: bool, z: u8, x: u32, y: u32) -> Option<(i64, Vec<u8>)> {
    sqlx::query_as("SELECT fetched_at_ms, body FROM traffic_tiles WHERE dark = ? AND z = ? AND x = ? AND y = ?")
        .bind(dark)
        .bind(z)
        .bind(x)
        .bind(y)
        .fetch_optional(pool)
        .await
        .inspect_err(|e| tracing::warn!(error = %e, "Verkehrskachel nicht lesbar"))
        .ok()?
}

/// Zählt eine Abfrage bei TomTom; `false`, wenn die Tagesgrenze erreicht ist.
pub async fn take_budget(pool: &SqlitePool, now_ms: i64) -> Result<bool, AppError> {
    let day = DateTime::<Utc>::from_timestamp_millis(now_ms)
        .unwrap_or_default()
        .format("%Y-%m-%d")
        .to_string();
    let taken: Option<i64> = sqlx::query_scalar(
        "INSERT INTO traffic_budget (day, tiles) VALUES (?, 1)
         ON CONFLICT(day) DO UPDATE SET tiles = tiles + 1 WHERE tiles < ?
         RETURNING tiles",
    )
    .bind(day)
    .bind(TILES_PER_DAY)
    .fetch_optional(pool)
    .await?;
    Ok(taken.is_some())
}

pub async fn flow_tile(
    State(state): State<AppState>,
    Path((theme, z, x, y)): Path<(String, u8, u32, u32)>,
) -> Result<Response, AppError> {
    let dark = match theme.as_str() {
        "dunkel" => true,
        "hell" => false,
        _ => return Err(AppError::NotFound),
    };
    let size = 1u32.checked_shl(z.into()).unwrap_or(0);
    if z > MAX_ZOOM || x >= size || y >= size {
        return Err(AppError::NotFound);
    }
    let cfg = settings::load(&state.pool).await?;
    let Some(key) = cfg.traffic.key_for(cfg.traffic.flow) else {
        return Err(AppError::NotFound);
    };
    let now = Utc::now().timestamp_millis();
    if let Some((at, body)) = cached(&state.pool, dark, z, x, y).await
        && now - at < FLOW_TTL_MS
    {
        return Ok(png(body));
    }
    // Nacheinander, und nach dem Warten noch einmal nachsehen: Ein anderer Browser hat sie vielleicht schon geholt.
    let _one_at_a_time = state.traffic_tile_fetch.lock().await;
    let old = cached(&state.pool, dark, z, x, y).await;
    if let Some((at, _)) = &old
        && now - at < FLOW_TTL_MS
    {
        return Ok(png(old.unwrap().1));
    }
    let fallback = || {
        old.clone()
            .map(|(_, body)| png(body))
            .ok_or(AppError::NotFound)
    };
    if !take_budget(&state.pool, now).await? {
        tracing::debug!("Tagesgrenze für Verkehrskacheln erreicht");
        return fallback();
    }
    let url = tomtom::flow_tile_url(&state.tomtom_base, key, dark, z, x, y)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    match tomtom::get(&state.http, url).await {
        Ok(body) => {
            sqlx::query(
                "INSERT INTO traffic_tiles (dark, z, x, y, fetched_at_ms, body) VALUES (?, ?, ?, ?, ?, ?)
                 ON CONFLICT(dark, z, x, y) DO UPDATE SET fetched_at_ms = excluded.fetched_at_ms, body = excluded.body",
            )
            .bind(dark)
            .bind(z)
            .bind(x)
            .bind(y)
            .bind(now)
            .bind(&body)
            .execute(&state.pool)
            .await?;
            state.errors.write().await.remove(FLOW_KEY);
            Ok(png(body))
        }
        Err(e) => {
            tracing::warn!(error = %e, "Verkehrskachel nicht abgerufen");
            state
                .errors
                .write()
                .await
                .insert(FLOW_KEY.into(), "Verkehrsfluss nicht abrufbar".into());
            fallback()
        }
    }
}

/// Räumt alte Kacheln und Tageszähler weg; läuft mit `poller::poll_traffic`.
pub async fn prune(pool: &SqlitePool, now_ms: i64) -> Result<(), AppError> {
    sqlx::query("DELETE FROM traffic_tiles WHERE fetched_at_ms < ?")
        .bind(now_ms - 24 * 3_600_000)
        .execute(pool)
        .await?;
    let week_ago = DateTime::<Utc>::from_timestamp_millis(now_ms - 7 * 86_400_000)
        .unwrap_or_default()
        .format("%Y-%m-%d")
        .to_string();
    sqlx::query("DELETE FROM traffic_budget WHERE day < ?")
        .bind(week_ago)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::TrafficLink;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::{Router, routing::get};
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tower::ServiceExt;

    async fn fake_tomtom(calls: Arc<AtomicUsize>) -> reqwest::Url {
        let app = Router::new().route(
            "/traffic/map/4/tile/flow/{style}/{z}/{x}/{file}",
            get(move || {
                let calls = Arc::clone(&calls);
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    b"PNG".to_vec()
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        url.parse().unwrap()
    }

    async fn state_with(traffic: TrafficLink, base: reqwest::Url) -> AppState {
        let boot = crate::config::BootConfig::from_lookup(|_| None).unwrap();
        let state = AppState::new(crate::db::test_pool().await, Arc::new(boot)).await;
        let mut cfg = settings::load(&state.pool).await.unwrap();
        cfg.traffic = traffic;
        settings::save(&state.pool, &cfg).await.unwrap();
        AppState {
            tomtom_base: base,
            ..state
        }
    }

    async fn tile(state: &AppState, path: &str) -> (StatusCode, Vec<u8>) {
        let res = crate::app::router(state.clone())
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        (
            status,
            res.into_body().collect().await.unwrap().to_bytes().to_vec(),
        )
    }

    fn on() -> TrafficLink {
        TrafficLink {
            tomtom_key: Some("schluessel0123456789".into()),
            flow: true,
            incidents: false,
            speed_cameras: false,
        }
    }

    #[tokio::test]
    async fn a_tile_is_fetched_once_and_then_served_from_the_cache() {
        let calls = Arc::new(AtomicUsize::new(0));
        let state = state_with(on(), fake_tomtom(Arc::clone(&calls)).await).await;
        let (status, body) = tile(&state, "/api/verkehr/fluss/dunkel/10/543/330").await;
        assert_eq!((status, body.as_slice()), (StatusCode::OK, &b"PNG"[..]));
        tile(&state, "/api/verkehr/fluss/dunkel/10/543/330").await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        tile(&state, "/api/verkehr/fluss/hell/10/543/330").await;
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "hell ist eine eigene Kachel"
        );
    }

    #[tokio::test]
    async fn nothing_without_key_or_with_flow_off() {
        let calls = Arc::new(AtomicUsize::new(0));
        let base = fake_tomtom(Arc::clone(&calls)).await;
        let off = TrafficLink {
            flow: false,
            ..on()
        };
        let state = state_with(off, base.clone()).await;
        assert_eq!(
            tile(&state, "/api/verkehr/fluss/hell/1/0/0").await.0,
            StatusCode::NOT_FOUND
        );
        let keyless = TrafficLink {
            tomtom_key: None,
            ..on()
        };
        let state = state_with(keyless, base).await;
        assert_eq!(
            tile(&state, "/api/verkehr/fluss/hell/1/0/0").await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn bad_paths_are_not_found() {
        let calls = Arc::new(AtomicUsize::new(0));
        let state = state_with(on(), fake_tomtom(Arc::clone(&calls)).await).await;
        for path in [
            "/api/verkehr/fluss/bunt/1/0/0",
            "/api/verkehr/fluss/hell/1/2/0",
            "/api/verkehr/fluss/hell/19/0/0",
        ] {
            assert_eq!(tile(&state, path).await.0, StatusCode::NOT_FOUND, "{path}");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn the_daily_limit_holds_and_an_old_tile_stays() {
        let calls = Arc::new(AtomicUsize::new(0));
        let state = state_with(on(), fake_tomtom(Arc::clone(&calls)).await).await;
        tile(&state, "/api/verkehr/fluss/hell/2/1/1").await;
        // Kachel veraltet, Tagesgrenze erreicht.
        sqlx::query("UPDATE traffic_tiles SET fetched_at_ms = 0")
            .execute(&state.pool)
            .await
            .unwrap();
        sqlx::query("UPDATE traffic_budget SET tiles = ?")
            .bind(TILES_PER_DAY)
            .execute(&state.pool)
            .await
            .unwrap();
        let (status, body) = tile(&state, "/api/verkehr/fluss/hell/2/1/1").await;
        assert_eq!(
            (status, body.as_slice()),
            (StatusCode::OK, &b"PNG"[..]),
            "die alte Kachel"
        );
        assert_eq!(
            tile(&state, "/api/verkehr/fluss/hell/2/0/0").await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn budget_counts_per_day() {
        let pool = crate::db::test_pool().await;
        let day = 1_793_000_000_000;
        sqlx::query("INSERT INTO traffic_budget (day, tiles) VALUES ('2026-10-27', ?)")
            .bind(TILES_PER_DAY)
            .execute(&pool)
            .await
            .unwrap();
        assert!(take_budget(&pool, day).await.unwrap(), "ein neuer Tag");
        prune(&pool, day + 9 * 86_400_000).await.unwrap();
        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM traffic_budget")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(left, 0);
    }
}
