//! Abruf im Hintergrund: je Quelle eine Schleife, Ergebnis landet in `snapshots`.

use std::time::Duration;

use chrono::Utc;
use reqwest::Url;
use serde::Serialize;

use crate::app::AppState;
use crate::app::SuchlageSnapshot;
use crate::contract::{Location, Region, Warning};
use crate::sources::{SourceError, nina, osm, suchlage, suite, tomtom, weather};
use crate::{settings, store};

pub const NINA_BASE: &str = "https://warnung.bund.de/api31";
pub const BRIGHT_SKY_BASE: &str = "https://api.brightsky.dev";
pub const NINA_INTERVAL: Duration = Duration::from_secs(60);
pub const WEATHER_INTERVAL: Duration = Duration::from_secs(600);
pub const CONFIG_INTERVAL: Duration = Duration::from_secs(60);
pub const SUCHLAGE_INTERVAL: Duration = Duration::from_secs(5);
/// Takt, in dem nachgesehen wird, ob ein Verkehrs-Stand fällig ist; abgefragt wird viel seltener.
pub const TRAFFIC_TICK: Duration = Duration::from_secs(60);
/// Störungen stündlich: 2 Instanzen × 24 × 31 = 1.488 von 2.500 Abfragen im Monat (`sources::tomtom`).
pub const INCIDENTS_INTERVAL_MS: i64 = 60 * 60_000;
/// Feste Blitzer ändern sich selten.
pub const SPEED_CAMERA_INTERVAL_MS: i64 = 24 * 60 * 60_000;
/// Nach einem Fehlschlag frühestens so viel später noch einmal.
pub const TRAFFIC_RETRY_MS: i64 = 15 * 60_000;

/// Stand-Schlüssel der Konfiguration aus der Suite (nur Zeitpunkt, die Werte stehen in `settings`).
pub const CONFIG_KEY: &str = "suite-config";
pub const SUCHLAGE_KEY: &str = "suchlage";

/// Schlüssel der Stände: Ort und Kreis gehören dazu, sonst stünde nach einer Änderung der
/// Einstellungen der alte Stand unter dem neuen Namen.
pub fn nina_key(region: &Region) -> String {
    format!("nina:{}", region.ars)
}

pub fn weather_key(location: &Location) -> String {
    format!("weather:{:.4},{:.4}", location.lat, location.lon)
}

pub fn incidents_key(location: &Location) -> String {
    format!("tomtom:{:.4},{:.4}", location.lat, location.lon)
}

pub fn speed_cameras_key(location: &Location) -> String {
    format!("blitzer:{:.4},{:.4}", location.lat, location.lon)
}

/// Fällig, wenn der Stand älter als `interval_ms` ist (oder fehlt) und der letzte Versuch länger als
/// `TRAFFIC_RETRY_MS` her. Der Stand liegt in `snapshots`: Ein Neustart fragt nicht neu.
pub async fn traffic_due(state: &AppState, key: &str, interval_ms: i64, now_ms: i64) -> bool {
    let fresh = store::fetched_at(&state.pool, key)
        .await
        .is_some_and(|at| now_ms - at < interval_ms);
    let mut attempts = state.traffic_attempts.lock().await;
    let recently_tried = attempts
        .get(key)
        .is_some_and(|&at| now_ms - at < TRAFFIC_RETRY_MS);
    if fresh || recently_tried {
        return false;
    }
    attempts.insert(key.to_string(), now_ms);
    true
}

/// Ein Abruf. Erfolg speichert den Stand und löscht den Fehler; Fehler lässt den alten Stand stehen.
/// Der Abruf läuft in eigenem Task: Eine Panik wird zum Fehlereintrag, die Schleife läuft weiter.
/// Nach außen (`errors` → `/api/monitor`) gehen nur kurze deutsche Texte, Details nur ins Log.
/// `clock` läuft erst nach dem Abruf: Der Stand ist so alt wie seine Daten, nicht wie sein Start.
pub async fn run_once<T, F, Fut>(
    state: &AppState,
    key: &str,
    fetcher: F,
    clock: impl FnOnce() -> i64,
) where
    T: Serialize + Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, SourceError>> + Send + 'static,
{
    let outcome = match tokio::spawn(async move { fetcher().await }).await {
        Ok(Ok(data)) => match crate::store::save(&state.pool, key, clock(), &data).await {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::warn!(key, error = ?e, "Speichern fehlgeschlagen");
                Err("Speichern fehlgeschlagen".to_string())
            }
        },
        Ok(Err(e)) => {
            tracing::warn!(key, error = %e, "Abruf fehlgeschlagen");
            Err(e.to_string())
        }
        Err(e) => {
            tracing::error!(key, error = %e, "Abruf abgestürzt");
            Err("Abruf abgestürzt".to_string())
        }
    };
    let mut errors = state.errors.write().await;
    match outcome {
        Ok(()) => errors.remove(key),
        Err(m) => errors.insert(key.to_string(), m),
    };
}

/// Einstellungen je Lauf neu lesen: Änderungen wirken ohne Neustart. Sind sie nicht lesbar,
/// meldet `/api/monitor` das selbst (gleiche Funktion), hier genügt das Log.
async fn load_settings(state: &AppState) -> Option<settings::Settings> {
    settings::load(&state.pool)
        .await
        .inspect_err(|e| tracing::warn!(error = ?e, "Einstellungen nicht lesbar"))
        .ok()
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

pub async fn poll_nina(state: &AppState, http: &reqwest::Client, base: &Url) {
    let Some(cfg) = load_settings(state).await else {
        return;
    };
    let key = nina_key(&cfg.region);
    // Ist eine einzelne Warnung nicht abrufbar, gilt ihr letzter Stand (siehe `nina::fetch`).
    let previous = store::load::<Vec<Warning>>(&state.pool, &key)
        .await
        .map(|(_, warnings)| warnings)
        .unwrap_or_default();
    let (http, base) = (http.clone(), base.clone());
    let fetcher =
        move || async move { nina::fetch(&http, &base, &cfg.region.ars, &previous).await };
    run_once(state, &key, fetcher, now_ms).await;
}

pub async fn poll_weather(state: &AppState, http: &reqwest::Client, base: &Url) {
    let Some(cfg) = load_settings(state).await else {
        return;
    };
    let key = weather_key(&cfg.location);
    let (http, base) = (http.clone(), base.clone());
    let fetcher =
        move || async move { weather::fetch(&http, &base, &cfg.location, Utc::now()).await };
    run_once(state, &key, fetcher, now_ms).await;
}

/// Baustellen und Sperrungen von TomTom, feste Blitzer aus OSM (nur online), jeweils nur wenn fällig.
pub async fn poll_traffic(
    state: &AppState,
    http: &reqwest::Client,
    tomtom_base: &Url,
    overpass: &Url,
) {
    let now = now_ms();
    if let Err(e) = crate::web::traffic::prune(&state.pool, now).await {
        tracing::warn!(error = ?e, "alte Verkehrskacheln nicht gelöscht");
    }
    let Some(cfg) = load_settings(state).await else {
        return;
    };
    if let Some(key) = cfg.traffic.key_for(cfg.traffic.incidents) {
        let k = incidents_key(&cfg.location);
        if traffic_due(state, &k, INCIDENTS_INTERVAL_MS, now).await {
            let (http, base, key, loc) = (
                http.clone(),
                tomtom_base.clone(),
                key.to_string(),
                cfg.location.clone(),
            );
            let fetcher =
                move || async move { tomtom::fetch_incidents(&http, &base, &key, &loc).await };
            run_once(state, &k, fetcher, now_ms).await;
        }
    }
    if cfg.traffic.speed_cameras && state.boot.online() {
        let k = speed_cameras_key(&cfg.location);
        if traffic_due(state, &k, SPEED_CAMERA_INTERVAL_MS, now).await {
            let (http, base, loc) = (http.clone(), overpass.clone(), cfg.location.clone());
            let fetcher = move || async move { osm::fetch(&http, &base, &loc).await };
            run_once(state, &k, fetcher, now_ms).await;
        }
    }
}

/// Holt die Konfiguration aus der Suite und schreibt sie in `settings`. Scheitert der Abruf, gilt
/// der letzte Stand weiter, und der Fehler steht unter `CONFIG_KEY`.
pub async fn poll_config(state: &AppState) {
    let Some(link) = state.boot.suite.clone() else {
        return;
    };
    let http = state.http.clone();
    let fetched = tokio::spawn(async move { suite::fetch(&http, &link).await }).await;
    let outcome = match fetched {
        Ok(Ok(cfg)) => match settings::save(&state.pool, &cfg).await {
            Ok(()) => store::save(&state.pool, CONFIG_KEY, now_ms(), &())
                .await
                .map_err(|e| {
                    tracing::warn!(error = ?e, "Stand der Konfiguration nicht gespeichert");
                    "Speichern fehlgeschlagen".to_string()
                }),
            Err(e) => {
                tracing::warn!(error = ?e, "Konfiguration nicht gespeichert");
                Err("Speichern fehlgeschlagen".to_string())
            }
        },
        Ok(Err(e)) => {
            tracing::warn!(error = %e, "Konfiguration nicht abgeholt");
            Err(e.to_string())
        }
        Err(e) => {
            tracing::error!(error = %e, "Abholen der Konfiguration abgestürzt");
            Err("Abruf abgestürzt".to_string())
        }
    };
    let mut errors = state.errors.write().await;
    match outcome {
        Ok(()) => errors.remove(CONFIG_KEY),
        Err(m) => errors.insert(CONFIG_KEY.to_string(), m),
    };
}

/// Fragt Suchlage, solange das Modul an und verbunden ist. Ein Fehler lässt den letzten Stand
/// stehen; er veraltet dann sichtbar (`web::monitor`).
pub async fn poll_suchlage(state: &AppState) {
    let Some(cfg) = load_settings(state).await else {
        return;
    };
    let link = match cfg.suchlage {
        Some(link) if cfg.modules.suchlage => link,
        _ => {
            *state.suchlage.write().await = None;
            state.errors.write().await.remove(SUCHLAGE_KEY);
            return;
        }
    };
    let (http, l) = (state.http.clone(), link.clone());
    let outcome = match tokio::spawn(async move { suchlage::fetch(&http, &l).await }).await {
        Ok(Ok(view)) => {
            *state.suchlage.write().await = Some(SuchlageSnapshot {
                url: link.url,
                fetched_at_ms: now_ms(),
                view,
            });
            Ok(())
        }
        Ok(Err(e)) => {
            tracing::debug!(error = %e, "Suchlage nicht abgefragt");
            Err(e.to_string())
        }
        Err(e) => {
            tracing::error!(error = %e, "Suchlage-Abfrage abgestürzt");
            Err("Abruf abgestürzt".to_string())
        }
    };
    let mut errors = state.errors.write().await;
    match outcome {
        Ok(()) => errors.remove(SUCHLAGE_KEY),
        Err(m) => errors.insert(SUCHLAGE_KEY.to_string(), m),
    };
}

pub fn spawn(state: AppState) {
    let s = state.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(CONFIG_INTERVAL);
        loop {
            tick.tick().await;
            poll_config(&s).await;
        }
    });
    let s = state.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(SUCHLAGE_INTERVAL);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            poll_suchlage(&s).await;
        }
    });

    let http = reqwest::Client::new();
    let nina_base: Url = NINA_BASE.parse().expect("gültige Konstante");
    let weather_base: Url = BRIGHT_SKY_BASE.parse().expect("gültige Konstante");

    let (s, h) = (state.clone(), http.clone());
    let overpass: Url = osm::OVERPASS_BASE.parse().expect("gültige Konstante");
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(TRAFFIC_TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            let base = s.tomtom_base.clone();
            poll_traffic(&s, &h, &base, &overpass).await;
        }
    });

    let (s, h) = (state.clone(), http.clone());
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(NINA_INTERVAL);
        loop {
            tick.tick().await;
            poll_nina(&s, &h, &nina_base).await;
        }
    });
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(WEATHER_INTERVAL);
        loop {
            tick.tick().await;
            poll_weather(&state, &http, &weather_base).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_carry_parameters() {
        let region = Region {
            ars: "031550000000".into(),
            name: "Landkreis Celle".into(),
        };
        let location = Location {
            lat: 52.965_61,
            lon: 10.5594,
            label: "Uelzen".into(),
        };
        assert_eq!(nina_key(&region), "nina:031550000000");
        assert_eq!(weather_key(&location), "weather:52.9656,10.5594");
        assert_eq!(incidents_key(&location), "tomtom:52.9656,10.5594");
    }

    async fn state() -> AppState {
        let boot = crate::config::BootConfig::from_lookup(|_| None).unwrap();
        AppState::new(crate::db::test_pool().await, std::sync::Arc::new(boot)).await
    }

    #[tokio::test]
    async fn traffic_is_due_rarely() {
        let s = state().await;
        let hour = INCIDENTS_INTERVAL_MS;
        assert!(
            traffic_due(&s, "tomtom:x", hour, 1_000_000).await,
            "noch nie gefragt"
        );
        assert!(
            !traffic_due(&s, "tomtom:x", hour, 1_060_000).await,
            "gescheitert: erst nach 15 min wieder"
        );
        assert!(traffic_due(&s, "tomtom:x", hour, 1_000_000 + TRAFFIC_RETRY_MS).await);
        store::save(&s.pool, "tomtom:x", 2_000_000, &())
            .await
            .unwrap();
        assert!(
            !traffic_due(&s, "tomtom:x", hour, 2_000_000 + TRAFFIC_RETRY_MS).await,
            "Stand frisch"
        );
        assert!(traffic_due(&s, "tomtom:x", hour, 2_000_000 + hour).await);
    }
}
