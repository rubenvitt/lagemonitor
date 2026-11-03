//! `GET /api/monitor`: alles, was die Oberfläche braucht, in einer Antwort.

use axum::Json;
use axum::extract::State;
use chrono::Utc;
use serde::de::DeserializeOwned;

use crate::app::AppState;
use crate::contract::{
    ConfigState, MapInfo, MonitorView, NOTICES_VISIBLE_MAX, Position, Sourced, SpeedCamera,
    SuchlageView, TrafficIncident, TrafficView, Warning, Weather,
};
use crate::error::AppError;
use crate::{poller, settings, store};

const NINA_STALE_MS: i64 = 5 * 60_000;
const WEATHER_STALE_MS: i64 = 45 * 60_000;
/// Drei verpasste Abfragen à 5 s.
const SUCHLAGE_STALE_MS: i64 = 20_000;
/// Ohne Einsatz fragt der Server nur alle 30 s (`poller::SUCHLAGE_IDLE_EVERY_MS`): drei verpasste.
const SUCHLAGE_IDLE_STALE_MS: i64 = 3 * poller::SUCHLAGE_IDLE_EVERY_MS;
/// gpsd meldet jede Sekunde, gespeichert wird alle 10 s: Eine Minute ohne Fix fällt auf.
const GPS_STALE_MS: i64 = 60_000;
/// Ein Fahrzeug ohne Meldung seit einem Tag verschwindet von der Karte (LGM-10, wie in der Suite).
const VEHICLE_HIDDEN_MS: i64 = 24 * 60 * 60_000;
/// Drei verpasste Abfragen; Baustellen ändern sich langsam, der Stand bleibt trotzdem sichtbar.
const INCIDENTS_STALE_MS: i64 = 3 * poller::INCIDENTS_INTERVAL_MS;
const SPEED_CAMERA_STALE_MS: i64 = 3 * poller::SPEED_CAMERA_INTERVAL_MS;

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

/// `None`, wenn das Modul aus oder nicht verbunden ist. Ein Stand von einer anderen Adresse zählt nicht.
async fn suchlage(
    state: &AppState,
    cfg: &settings::Settings,
    now_ms: i64,
) -> Option<Sourced<SuchlageView>> {
    let link = cfg.suchlage.as_ref().filter(|_| cfg.modules.suchlage)?;
    let error = state.errors.read().await.get(poller::SUCHLAGE_KEY).cloned();
    let snapshot = state
        .suchlage
        .read()
        .await
        .clone()
        .filter(|s| s.url == link.url);
    Some(match snapshot {
        Some(s) => {
            let max_age = if s.view.mission.is_some() {
                SUCHLAGE_STALE_MS
            } else {
                SUCHLAGE_IDLE_STALE_MS
            };
            Sourced {
                stale: now_ms - s.fetched_at_ms > max_age,
                fetched_at_ms: Some(s.fetched_at_ms),
                data: Some(s.view),
                error,
            }
        }
        None => Sourced {
            data: None,
            fetched_at_ms: None,
            stale: true,
            error,
        },
    })
}

/// Ebenen ohne Schlüssel oder ausgeschaltet kommen als `false`/`null`; Blitzer nur online.
async fn traffic(state: &AppState, cfg: &settings::Settings, now_ms: i64) -> TrafficView {
    let t = &cfg.traffic;
    let incidents = match t.key_for(t.incidents) {
        Some(_) => Some(
            sourced::<Vec<TrafficIncident>>(
                state,
                &poller::incidents_key(&poller::traffic_location(&cfg.location)),
                INCIDENTS_STALE_MS,
                now_ms,
            )
            .await,
        ),
        None => None,
    };
    let speed_cameras = if t.speed_cameras && state.boot.online() {
        Some(
            sourced::<Vec<SpeedCamera>>(
                state,
                &poller::speed_cameras_key(&poller::traffic_location(&cfg.location)),
                SPEED_CAMERA_STALE_MS,
                now_ms,
            )
            .await,
        )
    } else {
        None
    };
    let mut flow_error = state
        .errors
        .read()
        .await
        .get(crate::web::traffic::FLOW_KEY)
        .cloned();
    let flow = t.key_for(t.flow).is_some();
    if !flow {
        flow_error = None;
    }
    TrafficView {
        flow,
        flow_error,
        incidents,
        speed_cameras,
    }
}

async fn config(state: &AppState) -> ConfigState {
    ConfigState {
        from_suite: state.boot.suite.is_some(),
        fetched_at_ms: store::load::<()>(&state.pool, poller::CONFIG_KEY)
            .await
            .map(|(at, ())| at),
        error: state.errors.read().await.get(poller::CONFIG_KEY).cloned(),
    }
}

pub async fn get(State(state): State<AppState>) -> Result<Json<MonitorView>, AppError> {
    let cfg = settings::load(&state.pool).await?;
    let now_ms = Utc::now().timestamp_millis();
    let location = poller::location(&state, &cfg).await;
    let gps = match state.boot.gpsd {
        Some(_) => Some(sourced::<Position>(&state, poller::GPS_KEY, GPS_STALE_MS, now_ms).await),
        None => None,
    };
    let (weather_key, nina_key) = (
        poller::weather_key_near(&state, &location).await,
        poller::nina_key(&cfg.region),
    );
    let suchlage = suchlage(&state, &cfg, now_ms).await;
    let config = config(&state).await;
    let traffic = traffic(&state, &cfg, now_ms).await;
    Ok(Json(MonitorView {
        now_ms,
        location,
        gps,
        region: cfg.region,
        warn_threshold: cfg.warn_threshold,
        weather: sourced::<Weather>(&state, &weather_key, WEATHER_STALE_MS, now_ms).await,
        warnings: sourced::<Vec<Warning>>(&state, &nina_key, NINA_STALE_MS, now_ms).await,
        map: MapInfo {
            offline: state.tiles.is_some(),
            zoom: cfg.map_zoom,
        },
        modules: cfg.modules,
        config,
        suchlage,
        // Abgelaufenes geht nicht hinaus, auch wenn die Suite länger nicht erreichbar war; Geplantes erst ab
        // seinem Beginn. Die Suite schickt in der Reihenfolge, in der die Meldungen am Band stehen sollen.
        notices: cfg
            .notices
            .into_iter()
            .filter(|n| n.is_shown(now_ms))
            .take(NOTICES_VISIBLE_MAX)
            .collect(),
        recruitment: cfg.recruitment,
        traffic,
        // Wie die Suite: nach einem Tag ohne Meldung ist ein Fahrzeug weg, auch ohne neue Abholung. Ein fester
        // Ort meldet nichts und bleibt.
        vehicles: cfg
            .vehicles
            .into_iter()
            .filter(|v| v.fixed || now_ms - v.at_ms <= VEHICLE_HIDDEN_MS)
            .collect(),
    }))
}
