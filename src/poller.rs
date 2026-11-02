//! Abruf im Hintergrund: je Quelle eine Schleife, Ergebnis landet in `snapshots`.

use std::net::SocketAddr;
use std::time::Duration;

use chrono::Utc;
use reqwest::Url;
use serde::Serialize;

use crate::app::AppState;
use crate::app::SuchlageSnapshot;
use crate::contract::{Location, Position, Region, Warning};
use crate::sources::{SourceError, gpsd, nina, suchlage, suite, weather};
use crate::{settings, store};

pub const NINA_BASE: &str = "https://warnung.bund.de/api31";
pub const BRIGHT_SKY_BASE: &str = "https://api.brightsky.dev";
pub const NINA_INTERVAL: Duration = Duration::from_secs(60);
/// So oft wird nachgesehen, ob für den Standort ein frischer Wetterstand da ist: Fährt das Fahrzeug,
/// kommt der neue Ort binnen einer Minute an.
pub const WEATHER_INTERVAL: Duration = Duration::from_secs(60);
/// So alt darf ein Wetterstand werden, bevor er neu abgerufen wird.
const WEATHER_REFRESH_MS: i64 = 600_000;
const GPS_RETRY: Duration = Duration::from_secs(5);
const GPS_READ_TIMEOUT: Duration = Duration::from_secs(10);
/// gpsd meldet jede Sekunde; in die Datenbank geht höchstens alle 10 s ein Fix.
const GPS_SAVE_EVERY_MS: i64 = 10_000;
pub const CONFIG_INTERVAL: Duration = Duration::from_secs(60);
pub const SUCHLAGE_INTERVAL: Duration = Duration::from_secs(5);

/// Stand-Schlüssel der Konfiguration aus der Suite (nur Zeitpunkt, die Werte stehen in `settings`).
pub const CONFIG_KEY: &str = "suite-config";
pub const SUCHLAGE_KEY: &str = "suchlage";
/// Letzter GPS-Fix (`Position`).
pub const GPS_KEY: &str = "gps";

/// Schlüssel der Stände: Ort und Kreis gehören dazu, sonst stünde nach einer Änderung der
/// Einstellungen der alte Stand unter dem neuen Namen.
pub fn nina_key(region: &Region) -> String {
    format!("nina:{}", region.ars)
}

/// Auf rund einen Kilometer: Das GPS zittert um Meter, und je Schlüssel gibt es einen eigenen Stand.
pub fn weather_key(location: &Location) -> String {
    format!("weather:{:.2},{:.2}", location.lat, location.lon)
}

/// So weit darf ein Wetterstand weg sein, um bis zum nächsten Abruf einzuspringen.
const WEATHER_NEAR_KM: f64 = 25.0;

/// Schlüssel des Wetterstands für die Anzeige: der eigene, sonst der neueste in der Nähe. Fährt das
/// Fahrzeug, liegt der letzte Abruf meist eine Zelle zurück; ohne Rückfall wäre das Wetter leer.
/// Ein Ort weiter weg (geänderte Einstellungen) zeigt kein fremdes Wetter.
// shortcut: alte `weather:`-Zeilen bleiben liegen (eine je befahrener Zelle); aufräumen, wenn die Tabelle stört.
pub async fn weather_key_near(state: &AppState, location: &Location) -> String {
    let key = weather_key(location);
    let rows = store::sources_with_prefix(&state.pool, "weather:").await;
    if rows.iter().any(|(source, _)| *source == key) {
        return key;
    }
    let near = |source: &str| {
        let (lat, lon) = source.strip_prefix("weather:")?.split_once(',')?;
        let (lat, lon): (f64, f64) = (lat.parse().ok()?, lon.parse().ok()?);
        // Flache Näherung genügt auf diese Entfernung.
        let dy = (lat - location.lat) * 111.2;
        let dx = (lon - location.lon) * 111.2 * location.lat.to_radians().cos();
        Some(dx.hypot(dy) <= WEATHER_NEAR_KM)
    };
    rows.into_iter()
        .find(|(source, _)| near(source) == Some(true))
        .map_or(key, |(source, _)| source)
}

/// Standort für Wetter, Karte und Tageslicht: der letzte GPS-Fix, auch ein alter (das Fahrzeug steht
/// meist noch da). Ohne gpsd oder ohne je einen Fix der eingestellte Ort. Der Name bleibt der
/// eingestellte, einen Ortsnamen aus Koordinaten gibt es nicht.
pub async fn location(state: &AppState, cfg: &settings::Settings) -> Location {
    let fix = match state.boot.gpsd {
        Some(_) => store::load::<Position>(&state.pool, GPS_KEY).await,
        None => None,
    };
    match fix {
        Some((_, p)) => Location {
            lat: p.lat,
            lon: p.lon,
            label: cfg.location.label.clone(),
        },
        None => cfg.location.clone(),
    }
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
    let location = location(state, &cfg).await;
    let key = weather_key(&location);
    let fresh = store::load::<serde::de::IgnoredAny>(&state.pool, &key)
        .await
        .is_some_and(|(at, _)| now_ms() - at < WEATHER_REFRESH_MS);
    if fresh {
        return;
    }
    let (http, base) = (http.clone(), base.clone());
    let fetcher = move || async move { weather::fetch(&http, &base, &location, Utc::now()).await };
    run_once(state, &key, fetcher, now_ms).await;
}

/// Hält die Verbindung zum gpsd und baut sie nach einem Abbruch neu auf. Ohne Fix bleibt der letzte
/// Stand stehen und veraltet sichtbar (`web::monitor`); der Grund steht unter `GPS_KEY`.
pub async fn watch_gps(state: &AppState, addr: SocketAddr) {
    loop {
        let e = follow_gps(state, addr).await;
        tracing::debug!(error = %e, "gpsd-Verbindung beendet");
        state
            .errors
            .write()
            .await
            .insert(GPS_KEY.to_string(), e.to_string());
        tokio::time::sleep(GPS_RETRY).await;
    }
}

/// Eine Verbindung, bis sie abbricht.
async fn follow_gps(state: &AppState, addr: SocketAddr) -> SourceError {
    let mut conn = match gpsd::Connection::open(addr, GPS_READ_TIMEOUT).await {
        Ok(conn) => conn,
        Err(e) => return e,
    };
    let mut saved_at: Option<i64> = None;
    loop {
        let position = match conn.next().await {
            Ok(gpsd::Report::Fix(position)) => position,
            Ok(gpsd::Report::NoFix) => {
                let mut errors = state.errors.write().await;
                errors.insert(GPS_KEY.to_string(), "kein GPS-Fix".to_string());
                continue;
            }
            Err(e) => return e,
        };
        let now = now_ms();
        if saved_at.is_some_and(|at| now - at < GPS_SAVE_EVERY_MS) {
            continue;
        }
        let outcome = store::save(&state.pool, GPS_KEY, now, &position).await;
        let mut errors = state.errors.write().await;
        match outcome {
            Ok(()) => {
                saved_at = Some(now);
                errors.remove(GPS_KEY);
            }
            Err(e) => {
                tracing::warn!(error = ?e, "GPS-Fix nicht gespeichert");
                errors.insert(GPS_KEY.to_string(), "Speichern fehlgeschlagen".to_string());
            }
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
    if let Some(addr) = state.boot.gpsd {
        let s = state.clone();
        tokio::spawn(async move { watch_gps(&s, addr).await });
    }
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
        assert_eq!(weather_key(&location), "weather:52.97,10.56");
    }

    async fn gps_state(addr: Option<SocketAddr>) -> AppState {
        let boot = crate::config::BootConfig::from_lookup(move |k| {
            (k == "LAGEMONITOR_GPSD").then(|| addr.map(|a| a.to_string()))?
        })
        .unwrap();
        AppState::new(crate::db::test_pool().await, std::sync::Arc::new(boot)).await
    }

    #[tokio::test]
    async fn gps_fix_becomes_location_and_no_fix_is_reported() {
        use crate::sources::gpsd::tests::{FIX, NO_FIX, VERSION, fake_gpsd};
        let addr = fake_gpsd(&[VERSION, NO_FIX, FIX, NO_FIX], Duration::ZERO).await;
        let state = gps_state(Some(addr)).await;
        let ended = follow_gps(&state, addr).await;
        assert!(ended.to_string().contains("beendet"), "{ended}");
        assert_eq!(
            state.errors.read().await.get(GPS_KEY).map(String::as_str),
            Some("kein GPS-Fix")
        );

        let cfg = settings::load(&state.pool).await.unwrap();
        let loc = location(&state, &cfg).await;
        assert_eq!((loc.lat, loc.lon), (52.963709, 10.577172));
        assert_eq!(loc.label, cfg.location.label);

        let view = crate::web::monitor::get(axum::extract::State(state))
            .await
            .unwrap()
            .0;
        let gps = view.gps.expect("gpsd eingestellt");
        assert!(!gps.stale);
        assert_eq!(gps.error.as_deref(), Some("kein GPS-Fix"));
        assert_eq!(view.location, loc);
    }

    #[tokio::test]
    async fn weather_falls_back_to_nearby_cell() {
        let state = gps_state(None).await;
        let at = |lat: f64, lon: f64| Location {
            lat,
            lon,
            label: "x".into(),
        };
        // Zwei Zellen zurück (gut 2 km) liegt ein Stand, 60 km weg ein neuerer.
        store::save(&state.pool, "weather:52.97,10.56", 1000, &1)
            .await
            .unwrap();
        store::save(&state.pool, "weather:53.50,10.56", 2000, &2)
            .await
            .unwrap();
        let here = at(52.99, 10.57);
        assert_eq!(weather_key_near(&state, &here).await, "weather:52.97,10.56");
        // Gibt es den eigenen, gilt er.
        store::save(&state.pool, &weather_key(&here), 500, &3)
            .await
            .unwrap();
        assert_eq!(weather_key_near(&state, &here).await, weather_key(&here));
        // Weit weg nichts in der Nähe: der eigene Schlüssel, also kein Stand.
        let far = at(48.1, 11.6);
        assert_eq!(weather_key_near(&state, &far).await, weather_key(&far));
    }

    #[tokio::test]
    async fn without_gpsd_old_fix_is_ignored() {
        let state = gps_state(None).await;
        let fix = Position { lat: 1.0, lon: 2.0 };
        store::save(&state.pool, GPS_KEY, now_ms(), &fix)
            .await
            .unwrap();
        let cfg = settings::load(&state.pool).await.unwrap();
        assert_eq!(location(&state, &cfg).await, cfg.location);
        let view = crate::web::monitor::get(axum::extract::State(state))
            .await
            .unwrap()
            .0;
        assert!(view.gps.is_none());
    }
}
