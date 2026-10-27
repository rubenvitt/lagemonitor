//! Abruf im Hintergrund: je Quelle eine Schleife, Ergebnis landet in `snapshots`.

use std::time::Duration;

use chrono::Utc;
use reqwest::Url;
use serde::Serialize;

use crate::app::AppState;
use crate::settings;
use crate::sources::{SourceError, nina, weather};

pub const NINA_BASE: &str = "https://warnung.bund.de/api31";
pub const BRIGHT_SKY_BASE: &str = "https://api.brightsky.dev";
pub const NINA_INTERVAL: Duration = Duration::from_secs(60);
pub const WEATHER_INTERVAL: Duration = Duration::from_secs(600);

/// Ein Abruf. Erfolg speichert den Stand und löscht den Fehler; Fehler lässt den alten Stand stehen.
pub async fn run_once<T, F, Fut>(state: &AppState, source: &'static str, fetcher: F, now_ms: i64)
where
    T: Serialize,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, SourceError>>,
{
    let outcome = match fetcher().await {
        Ok(data) => crate::store::save(&state.pool, source, now_ms, &data)
            .await
            .map_err(|e| format!("Speichern fehlgeschlagen: {e:?}")),
        Err(e) => Err(e.to_string()),
    };
    let mut errors = state.errors.write().await;
    match outcome {
        Ok(()) => {
            errors.remove(source);
        }
        Err(message) => {
            tracing::warn!(source, error = %message, "Abruf fehlgeschlagen");
            errors.insert(source, message);
        }
    }
}

pub fn spawn(state: AppState) {
    let http = reqwest::Client::new();
    let nina_base: Url = NINA_BASE.parse().expect("gültige Konstante");
    let weather_base: Url = BRIGHT_SKY_BASE.parse().expect("gültige Konstante");

    let (s, h) = (state.clone(), http.clone());
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(NINA_INTERVAL);
        loop {
            tick.tick().await;
            // Einstellungen je Lauf neu lesen: Änderungen wirken ohne Neustart.
            let Ok(cfg) = settings::load(&s.pool).await else {
                continue;
            };
            let now_ms = Utc::now().timestamp_millis();
            run_once(
                &s,
                "nina",
                || nina::fetch(&h, &nina_base, &cfg.region.ars),
                now_ms,
            )
            .await;
        }
    });
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(WEATHER_INTERVAL);
        loop {
            tick.tick().await;
            let Ok(cfg) = settings::load(&state.pool).await else {
                continue;
            };
            let now = Utc::now();
            let fetcher = || weather::fetch(&http, &weather_base, &cfg.location, now);
            run_once(&state, "weather", fetcher, now.timestamp_millis()).await;
        }
    });
}
