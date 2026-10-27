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
/// Der Abruf läuft in eigenem Task: Eine Panik wird zum Fehlereintrag, die Schleife läuft weiter.
/// Nach außen (`errors` → `/api/monitor`) gehen nur kurze deutsche Texte, Details nur ins Log.
pub async fn run_once<T, F, Fut>(state: &AppState, source: &'static str, fetcher: F, now_ms: i64)
where
    T: Serialize + Send + 'static,
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T, SourceError>> + Send + 'static,
{
    let outcome = match tokio::spawn(async move { fetcher().await }).await {
        Ok(Ok(data)) => match crate::store::save(&state.pool, source, now_ms, &data).await {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::warn!(source, error = ?e, "Speichern fehlgeschlagen");
                Err("Speichern fehlgeschlagen".to_string())
            }
        },
        Ok(Err(e)) => {
            tracing::warn!(source, error = %e, "Abruf fehlgeschlagen");
            Err(e.to_string())
        }
        Err(e) => {
            tracing::error!(source, error = %e, "Abruf abgestürzt");
            Err("Abruf abgestürzt".to_string())
        }
    };
    set_error(state, source, outcome.err()).await;
}

async fn set_error(state: &AppState, source: &'static str, message: Option<String>) {
    let mut errors = state.errors.write().await;
    match message {
        Some(m) => errors.insert(source, m),
        None => errors.remove(source),
    };
}

/// Einstellungen je Lauf neu lesen: Änderungen wirken ohne Neustart.
async fn load_settings(state: &AppState, source: &'static str) -> Option<settings::Settings> {
    match settings::load(&state.pool).await {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            tracing::warn!(source, error = ?e, "Einstellungen nicht lesbar");
            set_error(state, source, Some("Einstellungen nicht lesbar".into())).await;
            None
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
            let Some(cfg) = load_settings(&s, "nina").await else {
                continue;
            };
            let now_ms = Utc::now().timestamp_millis();
            let (h, base) = (h.clone(), nina_base.clone());
            let fetcher = move || async move { nina::fetch(&h, &base, &cfg.region.ars).await };
            run_once(&s, "nina", fetcher, now_ms).await;
        }
    });
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(WEATHER_INTERVAL);
        loop {
            tick.tick().await;
            let Some(cfg) = load_settings(&state, "weather").await else {
                continue;
            };
            let now = Utc::now();
            let (h, base) = (http.clone(), weather_base.clone());
            let fetcher =
                move || async move { weather::fetch(&h, &base, &cfg.location, now).await };
            run_once(&state, "weather", fetcher, now.timestamp_millis()).await;
        }
    });
}
