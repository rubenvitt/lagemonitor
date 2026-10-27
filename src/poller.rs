//! Abruf im Hintergrund: je Quelle eine Schleife, Ergebnis landet in `snapshots`.

use std::time::Duration;

use chrono::Utc;
use reqwest::Url;
use serde::Serialize;

use crate::app::AppState;
use crate::contract::{Location, Region, Warning};
use crate::sources::{SourceError, nina, weather};
use crate::{settings, store};

pub const NINA_BASE: &str = "https://warnung.bund.de/api31";
pub const BRIGHT_SKY_BASE: &str = "https://api.brightsky.dev";
pub const NINA_INTERVAL: Duration = Duration::from_secs(60);
pub const WEATHER_INTERVAL: Duration = Duration::from_secs(600);

/// Schlüssel der Stände: Ort und Kreis gehören dazu, sonst stünde nach einer Änderung der
/// Einstellungen der alte Stand unter dem neuen Namen.
pub fn nina_key(region: &Region) -> String {
    format!("nina:{}", region.ars)
}

pub fn weather_key(location: &Location) -> String {
    format!("weather:{:.4},{:.4}", location.lat, location.lon)
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

pub fn spawn(state: AppState) {
    let http = reqwest::Client::new();
    let nina_base: Url = NINA_BASE.parse().expect("gültige Konstante");
    let weather_base: Url = BRIGHT_SKY_BASE.parse().expect("gültige Konstante");

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
    }
}
