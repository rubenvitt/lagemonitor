//! `GET /api/monitor`: alles, was die Oberfläche braucht, in einer Antwort.

use axum::Json;
use axum::extract::State;
use chrono::Utc;
use serde::de::DeserializeOwned;

use crate::app::AppState;
use crate::contract::{MapInfo, MonitorView, Sourced, Warning, Weather};
use crate::error::AppError;
use crate::{poller, settings, store};

const NINA_STALE_MS: i64 = 5 * 60_000;
const WEATHER_STALE_MS: i64 = 45 * 60_000;

async fn sourced<T: DeserializeOwned>(
    state: &AppState,
    key: &str,
    max_age_ms: i64,
    now_ms: i64,
) -> Sourced<T> {
    let snapshot = store::load::<T>(&state.pool, key).await;
    let error = state.errors.read().await.get(key).cloned();
    match snapshot {
        Some((fetched_at_ms, data)) => Sourced {
            data: Some(data),
            fetched_at_ms: Some(fetched_at_ms),
            stale: now_ms - fetched_at_ms > max_age_ms,
            error,
        },
        None => Sourced {
            data: None,
            fetched_at_ms: None,
            stale: true,
            error,
        },
    }
}

pub async fn get(State(state): State<AppState>) -> Result<Json<MonitorView>, AppError> {
    let cfg = settings::load(&state.pool).await?;
    let now_ms = Utc::now().timestamp_millis();
    let (weather_key, nina_key) = (
        poller::weather_key(&cfg.location),
        poller::nina_key(&cfg.region),
    );
    Ok(Json(MonitorView {
        now_ms,
        location: cfg.location,
        region: cfg.region,
        warn_threshold: cfg.warn_threshold,
        weather: sourced::<Weather>(&state, &weather_key, WEATHER_STALE_MS, now_ms).await,
        warnings: sourced::<Vec<Warning>>(&state, &nina_key, NINA_STALE_MS, now_ms).await,
        map: MapInfo {
            offline: state.tiles.is_some(),
        },
    }))
}
