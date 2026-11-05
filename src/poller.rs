//! Abruf im Hintergrund: je Quelle eine Schleife, Ergebnis landet in `snapshots`.

use std::net::SocketAddr;
use std::time::Duration;

use chrono::Utc;
use reqwest::Url;
use serde::Serialize;

use crate::app::AppState;
use crate::app::SuchlageSnapshot;
use crate::contract::{Location, Position, Region, Warning};
use crate::sources::{SourceError, gpsd, nina, osm, suchlage, suite, tomtom, weather};
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
/// Ohne laufenden Einsatz reicht alle 30 s: Ein neuer Einsatz erscheint so spätestens nach einer
/// halben Minute, und die Abfragen im Leerlauf (den größten Teil des Tages) sinken auf ein Sechstel.
pub const SUCHLAGE_IDLE_EVERY_MS: i64 = 30_000;
/// Gebiete und Flugspuren höchstens alle 30 s und nur bei einer Änderung; die Drohnen kommen
/// weiter alle 5 s. Die Spur hinter einer fliegenden Drohne hängt so bis zu 30 s nach.
pub const SUCHLAGE_MAP_EVERY_MS: i64 = 30_000;
/// Takt, in dem nachgesehen wird, ob ein Verkehrs-Stand fällig ist; abgefragt wird viel seltener.
pub const TRAFFIC_TICK: Duration = Duration::from_secs(60);
/// Störungen stündlich: 2 Instanzen × 24 × 31 = 1.488 von 2.500 Abfragen im Monat (`sources::tomtom`);
/// fahrend höchstens vier je Stunde dazu (`TRAFFIC_REQUERY_KM`).
pub const INCIDENTS_INTERVAL_MS: i64 = 60 * 60_000;
/// Feste Blitzer ändern sich selten.
pub const SPEED_CAMERA_INTERVAL_MS: i64 = 24 * 60 * 60_000;
/// Nach einem Fehlschlag frühestens so viel später noch einmal.
pub const TRAFFIC_RETRY_MS: i64 = 15 * 60_000;

/// Stand-Schlüssel der Konfiguration aus der Suite (nur Zeitpunkt, die Werte stehen in `settings`).
pub const CONFIG_KEY: &str = "suite-config";
/// `ETag` der zuletzt gespeicherten Konfiguration (`suite::fetch`).
pub const CONFIG_ETAG_KEY: &str = "suite-config-etag";
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
        Some(distance_km(lat, lon, location) <= WEATHER_NEAR_KM)
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
    // Ein fester Ort bleibt fest, auch wenn der Rechner zufällig ein gpsd hat.
    if cfg.vehicles.iter().any(|v| v.own && v.fixed) {
        return cfg.location.clone();
    }
    match fix {
        Some((_, p)) => Location {
            lat: p.lat,
            lon: p.lon,
            label: cfg.location.label.clone(),
        },
        None => cfg.location.clone(),
    }
}

/// Rastermaß für den Ort einer neuen Verkehrsabfrage, rund 5 km: So zittert der Schlüssel nicht mit
/// dem GPS, und ein Fahrzeug, das an der Wache steht, fragt immer unter demselben Schlüssel.
const TRAFFIC_GRID_DEG: f64 = 0.05;

/// Erst wenn der jüngste Stand weiter als das vom Fahrzeug weg ist, wird neu abgefragt (LGM-11). Das
/// Gebiet einer Abfrage reicht 49,5 km in jede Richtung (`tomtom::bbox`): Es bleiben immer gut 24 km
/// Vorlauf, und bei 100 km/h kommen höchstens vier Abfragen je Fahrstunde dazu (TomTom zählt jede).
const TRAFFIC_REQUERY_KM: f64 = 25.0;
/// So weit darf der angezeigte Stand weg sein: bis kurz vor den Rand seines Gebiets. Weiter weg
/// gilt der eigene Rasterpunkt, bis zu dem es noch keinen Stand gibt.
const TRAFFIC_SHOWN_KM: f64 = 40.0;

/// Präfixe der Stände; dahinter steht der Mittelpunkt der Abfrage (`incidents_key`).
pub const INCIDENTS_PREFIX: &str = "tomtom:";
pub const SPEED_CAMERAS_PREFIX: &str = "blitzer:";

/// Der Ort, um den Baustellen und Blitzer abgefragt werden: `location` auf das Raster gerückt.
pub fn traffic_location(location: &Location) -> Location {
    let snap = |v: f64| (v / TRAFFIC_GRID_DEG).round() * TRAFFIC_GRID_DEG;
    Location {
        lat: snap(location.lat),
        lon: snap(location.lon),
        label: location.label.clone(),
    }
}

pub fn incidents_key(location: &Location) -> String {
    format!("{INCIDENTS_PREFIX}{:.4},{:.4}", location.lat, location.lon)
}

pub fn speed_cameras_key(location: &Location) -> String {
    format!(
        "{SPEED_CAMERAS_PREFIX}{:.4},{:.4}",
        location.lat, location.lon
    )
}

/// Flache Näherung, genügt auf diese Entfernungen.
fn distance_km(lat: f64, lon: f64, location: &Location) -> f64 {
    let dy = (lat - location.lat) * 111.2;
    let dx = (lon - location.lon) * 111.2 * location.lat.to_radians().cos();
    dx.hypot(dy)
}

/// Der jüngste Stand mit `prefix`, dessen Abfrage höchstens `max_km` vom Ort entfernt war, samt
/// Zeitpunkt. Ältere Stände in der Nähe zählen nicht: Der jüngste ist der, der zuletzt passte.
// shortcut: alte Zeilen bleiben liegen (eine je Abfragepunkt); aufräumen, wenn die Tabelle stört.
async fn traffic_near(
    state: &AppState,
    prefix: &str,
    location: &Location,
    max_km: f64,
) -> Option<(String, i64)> {
    let near = |source: &str| {
        let (lat, lon) = source.strip_prefix(prefix)?.split_once(',')?;
        let (lat, lon): (f64, f64) = (lat.parse().ok()?, lon.parse().ok()?);
        Some(distance_km(lat, lon, location) <= max_km)
    };
    store::sources_with_prefix(&state.pool, prefix)
        .await
        .into_iter()
        .find(|(source, _)| near(source) == Some(true))
}

/// Schlüssel des Verkehrsstands für die Anzeige: derselbe, den `traffic_due` frisch hält, bis der
/// nächste da ist ein weiter entfernter, sonst der eigene Rasterpunkt. Beim Fahren springt die
/// Anzeige so nicht mit jeder Rasterzelle ins Leere.
pub async fn traffic_key_shown(state: &AppState, prefix: &str, location: &Location) -> String {
    let near = match traffic_near(state, prefix, location, TRAFFIC_REQUERY_KM).await {
        Some(found) => Some(found),
        None => traffic_near(state, prefix, location, TRAFFIC_SHOWN_KM).await,
    };
    match near {
        Some((key, _)) => key,
        None => {
            let at = traffic_location(location);
            format!("{prefix}{:.4},{:.4}", at.lat, at.lon)
        }
    }
}

/// Der Schlüssel, unter dem jetzt abgefragt werden soll, oder `None`. Fällig, wenn es in
/// `TRAFFIC_REQUERY_KM` keinen Stand jünger als `interval_ms` gibt und der letzte Versuch dieser
/// Ebene länger als `TRAFFIC_RETRY_MS` her ist. Der Versuch zählt je Ebene, nicht je Ort: Ein
/// fahrendes Fahrzeug fragt nach einem Fehlschlag nicht in jeder neuen Rasterzelle gleich wieder.
/// Der Stand liegt in `snapshots`: Ein Neustart fragt nicht neu.
pub async fn traffic_due(
    state: &AppState,
    prefix: &str,
    location: &Location,
    interval_ms: i64,
    now_ms: i64,
) -> Option<Location> {
    let fresh = traffic_near(state, prefix, location, TRAFFIC_REQUERY_KM)
        .await
        .is_some_and(|(_, at)| now_ms - at < interval_ms);
    let mut attempts = state.traffic_attempts.lock().await;
    let recently_tried = attempts
        .get(prefix)
        .is_some_and(|&at| now_ms - at < TRAFFIC_RETRY_MS);
    if fresh || recently_tried {
        return None;
    }
    attempts.insert(prefix.to_string(), now_ms);
    Some(traffic_location(location))
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

pub fn now_ms() -> i64 {
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
    // Der eigene GPS-Fix wie beim Wetter: Er ist sofort da, der Ort aus der Suite erst nach einer
    // Runde über deren Abholung.
    let here = location(state, &cfg).await;
    if let Some(key) = cfg.traffic.key_for(cfg.traffic.incidents) {
        let due = traffic_due(state, INCIDENTS_PREFIX, &here, INCIDENTS_INTERVAL_MS, now).await;
        if let Some(loc) = due {
            let k = incidents_key(&loc);
            let (http, base, key) = (http.clone(), tomtom_base.clone(), key.to_string());
            let fetcher =
                move || async move { tomtom::fetch_incidents(&http, &base, &key, &loc).await };
            run_once(state, &k, fetcher, now_ms).await;
        }
    }
    if cfg.traffic.speed_cameras && state.boot.online() {
        let due = traffic_due(
            state,
            SPEED_CAMERAS_PREFIX,
            &here,
            SPEED_CAMERA_INTERVAL_MS,
            now,
        );
        if let Some(loc) = due.await {
            let k = speed_cameras_key(&loc);
            let (http, base) = (http.clone(), overpass.clone());
            let fetcher = move || async move { osm::fetch(&http, &base, &loc).await };
            run_once(state, &k, fetcher, now_ms).await;
        }
    }
}

/// Holt die Konfiguration aus der Suite und schreibt sie in `settings`. Scheitert der Abruf, gilt
/// der letzte Stand weiter, und der Fehler steht unter `CONFIG_KEY`.
/// Was ein Monitor mit GPS bei der Abholung mitmeldet: der letzte Fix, solange er nicht älter als
/// ein Tag ist (älter zeigt ihn die Suite ohnehin nicht mehr).
async fn own_position(state: &AppState) -> Option<String> {
    state.boot.gpsd?;
    let (at, p) = store::load::<Position>(&state.pool, GPS_KEY).await?;
    let age_s = (now_ms() - at) / 1000;
    (age_s <= OWN_POSITION_MAX_AGE_S).then(|| suite::position_header(&p, age_s))
}

const OWN_POSITION_MAX_AGE_S: i64 = 24 * 60 * 60;

/// ETag der Suite-Konfiguration samt der Suite, von der er stammt.
#[derive(serde::Serialize, serde::Deserialize)]
struct ConfigEtag {
    scope: String,
    etag: String,
}

/// Kennung der Suite aus Adresse und Schlüssel; der Schlüssel landet nur gehasht in der Datenbank.
pub fn config_etag_scope(link: &crate::config::SuiteLink) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(format!("{}\n{}", link.url, link.token).as_bytes());
    hex::encode(&digest[..8])
}

pub async fn poll_config(state: &AppState) {
    let Some(link) = state.boot.suite.clone() else {
        return;
    };
    let position = own_position(state).await;
    // Ohne gespeicherten Stand (frische Datenbank) zählt ein alter ETag nicht, und für eine andere
    // Suite (Adresse oder Schlüssel geändert) auch nicht.
    let scope = config_etag_scope(&link);
    let etag = match store::fetched_at(&state.pool, CONFIG_KEY).await {
        Some(_) => store::load::<ConfigEtag>(&state.pool, CONFIG_ETAG_KEY)
            .await
            .map(|(_, e)| e)
            .filter(|e| e.scope == scope && !e.etag.is_empty())
            .map(|e| e.etag),
        None => None,
    };
    let http = state.http.clone();
    let fetched =
        tokio::spawn(async move { suite::fetch(&http, &link, position, etag.as_deref()).await })
            .await;
    let saved = |r: Result<(), crate::error::AppError>| {
        r.map_err(|e| {
            tracing::warn!(error = ?e, "Stand der Konfiguration nicht gespeichert");
            "Speichern fehlgeschlagen".to_string()
        })
    };
    let outcome = match fetched {
        Ok(Ok(suite::Fetched::Changed(cfg, new_etag))) => {
            match settings::save(&state.pool, &cfg).await {
                Ok(()) => {
                    let etag = ConfigEtag {
                        scope,
                        etag: new_etag.unwrap_or_default(),
                    };
                    saved(store::save(&state.pool, CONFIG_ETAG_KEY, now_ms(), &etag).await).and(
                        saved(store::save(&state.pool, CONFIG_KEY, now_ms(), &()).await),
                    )
                }
                Err(e) => {
                    tracing::warn!(error = ?e, "Konfiguration nicht gespeichert");
                    Err("Speichern fehlgeschlagen".to_string())
                }
            }
        }
        Ok(Ok(suite::Fetched::Unchanged)) => {
            saved(store::save(&state.pool, CONFIG_KEY, now_ms(), &()).await)
        }
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
    let now = now_ms();
    let filter = (link.areas, link.tracks);
    let previous = state
        .suchlage
        .read()
        .await
        .clone()
        .filter(|p| p.url == link.url);
    let (wish, known) = match &previous {
        Some(p) => {
            if p.view.mission.is_none() && now - p.fetched_at_ms < SUCHLAGE_IDLE_EVERY_MS {
                return;
            }
            suchlage_map_wish(p, filter, now)
        }
        None => (suchlage::MapWish::Full, None),
    };
    let (http, l) = (state.http.clone(), link.clone());
    let fetch = async move { suchlage::fetch(&http, &l, &wish, known.as_ref()).await };
    let outcome = match tokio::spawn(fetch).await {
        Ok(Ok(fetched)) => {
            let map_checked_ms = map_checked_after(
                previous.as_ref(),
                fetched.view.mission.as_ref(),
                fetched.map_checked,
                now,
            );
            *state.suchlage.write().await = Some(SuchlageSnapshot {
                url: link.url,
                fetched_at_ms: now,
                view: fetched.view,
                map: fetched.map,
                map_checked_ms,
                map_filter: filter,
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

/// Was die nächste Suchlage-Abfrage an Karte will. Ohne bekannte Fassung oder nach geänderten
/// Schaltern die ganze, sonst alle `SUCHLAGE_MAP_EVERY_MS` nur bei Änderung, dazwischen keine.
/// Wann die Karte zuletzt geprüft war. Wechselt der Einsatz, ohne dass die Karte mitkam, gilt sie
/// als ungeprüft: Die nächste Abfrage fragt nach ihr, statt die alte Karte 30 s stehen zu lassen.
/// Eine bloß neuere Fassung im selben Einsatz (wachsende Flugspur) wartet dagegen den Takt ab.
pub fn map_checked_after(
    previous: Option<&SuchlageSnapshot>,
    mission: Option<&crate::contract::SuchlageMission>,
    map_checked: bool,
    now_ms: i64,
) -> i64 {
    match previous {
        _ if map_checked => now_ms,
        Some(p) if p.view.mission.as_ref() == mission => p.map_checked_ms,
        _ => now_ms - SUCHLAGE_MAP_EVERY_MS,
    }
}

pub fn suchlage_map_wish(
    previous: &SuchlageSnapshot,
    filter: (bool, bool),
    now_ms: i64,
) -> (suchlage::MapWish, Option<suchlage::KnownMap>) {
    match &previous.map {
        Some(map) if previous.map_filter == filter => {
            let wish = if now_ms - previous.map_checked_ms < SUCHLAGE_MAP_EVERY_MS {
                suchlage::MapWish::None
            } else {
                suchlage::MapWish::IfChanged(map.etag.clone())
            };
            (wish, Some(map.clone()))
        }
        _ => (suchlage::MapWish::Full, None),
    }
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

    let http = crate::http::client();
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
    fn the_map_is_asked_for_rarely_and_only_if_changed() {
        let known = suchlage::KnownMap {
            etag: "e1".into(),
            areas: serde_json::json!({}),
            tracks: serde_json::json!({}),
        };
        let snap = |map: Option<suchlage::KnownMap>, checked| SuchlageSnapshot {
            url: "u".into(),
            fetched_at_ms: 0,
            view: suchlage::parse(
                suchlage::tests::RUNNING.as_bytes(),
                &suchlage::tests::link(),
            )
            .unwrap(),
            map,
            map_checked_ms: checked,
            map_filter: (true, true),
        };
        let wish = |s: &SuchlageSnapshot, filter, now| suchlage_map_wish(s, filter, now).0;
        assert_eq!(
            wish(&snap(None, 0), (true, true), 1),
            suchlage::MapWish::Full
        );
        let s = snap(Some(known), 1_000);
        assert_eq!(wish(&s, (true, true), 2_000), suchlage::MapWish::None);
        assert_eq!(
            wish(&s, (true, true), 1_000 + SUCHLAGE_MAP_EVERY_MS),
            suchlage::MapWish::IfChanged("e1".into())
        );
        assert_eq!(
            wish(&s, (true, false), 2_000),
            suchlage::MapWish::Full,
            "Schalter geändert"
        );
        let mission = s.view.mission.clone();
        assert_eq!(
            map_checked_after(Some(&s), mission.as_ref(), false, 2_000),
            1_000
        );
        assert_eq!(
            map_checked_after(Some(&s), mission.as_ref(), true, 2_000),
            2_000
        );
        let other = mission.map(|m| crate::contract::SuchlageMission {
            name: "Neuer Einsatz".into(),
            ..m
        });
        let checked = map_checked_after(Some(&s), other.as_ref(), false, 2_000);
        let s = SuchlageSnapshot {
            map_checked_ms: checked,
            ..s
        };
        assert_eq!(
            wish(&s, (true, true), 2_000 + 5_000),
            suchlage::MapWish::IfChanged("e1".into()),
            "neuer Einsatz: Karte gleich nachfragen"
        );
    }

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
        assert_eq!(incidents_key(&location), "tomtom:52.9656,10.5594");
    }

    #[test]
    fn traffic_is_asked_on_a_grid_not_at_every_position() {
        let at = |lat: f64, lon: f64| Location {
            lat,
            lon,
            label: "x".into(),
        };
        let key = |lat, lon| incidents_key(&traffic_location(&at(lat, lon)));
        assert_eq!(key(52.9656, 10.5594), "tomtom:52.9500,10.5500");
        // Ein paar hundert Meter weiter: dieselbe Abfrage.
        assert_eq!(key(52.9656, 10.5594), key(52.9690, 10.5640));
        // Ein paar Kilometer weiter: eine neue.
        assert_ne!(key(52.9656, 10.5594), key(53.0100, 10.5594));
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

        // Steht der Monitor an einem festen Ort, gilt der, nicht das gpsd.
        let fixed = settings::Settings {
            vehicles: vec![crate::contract::Vehicle {
                id: "ort".into(),
                call_sign: "Unterkunft".into(),
                symbol: "<svg/>".into(),
                lat: cfg.location.lat,
                lon: cfg.location.lon,
                at_ms: 0,
                own: true,
                fixed: true,
            }],
            ..cfg.clone()
        };
        assert_eq!(location(&state, &fixed).await, cfg.location);

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

    async fn state() -> AppState {
        let boot = crate::config::BootConfig::from_lookup(|_| None).unwrap();
        AppState::new(crate::db::test_pool().await, std::sync::Arc::new(boot)).await
    }

    fn at(lat: f64, lon: f64) -> Location {
        Location {
            lat,
            lon,
            label: "x".into(),
        }
    }

    #[tokio::test]
    async fn traffic_is_due_rarely() {
        let s = state().await;
        let hour = INCIDENTS_INTERVAL_MS;
        let here = at(52.9656, 10.5594);
        let due = |now| traffic_due(&s, INCIDENTS_PREFIX, &here, hour, now);
        assert_eq!(
            due(1_000_000).await.map(|l| incidents_key(&l)).as_deref(),
            Some("tomtom:52.9500,10.5500"),
            "noch nie gefragt"
        );
        assert_eq!(
            due(1_060_000).await,
            None,
            "gescheitert: erst nach 15 min wieder"
        );
        assert!(due(1_000_000 + TRAFFIC_RETRY_MS).await.is_some());
        store::save(&s.pool, &incidents_key(&at(52.95, 10.55)), 2_000_000, &())
            .await
            .unwrap();
        assert_eq!(
            due(2_000_000 + TRAFFIC_RETRY_MS).await,
            None,
            "Stand frisch"
        );
        assert!(due(2_000_000 + hour).await.is_some());
    }

    #[tokio::test]
    async fn a_moving_vehicle_asks_again_only_far_from_the_last_query() {
        let s = state().await;
        let hour = INCIDENTS_INTERVAL_MS;
        let (now, later) = (10_000_000, 10_000_000 + TRAFFIC_RETRY_MS);
        let home = at(52.9656, 10.5594);
        store::save(&s.pool, &incidents_key(&traffic_location(&home)), now, &1)
            .await
            .unwrap();
        // Gut 20 km nördlich (über vier Rasterzellen): Der Stand von zu Hause reicht noch.
        let north_20 = at(53.15, 10.5594);
        assert_eq!(
            traffic_due(&s, INCIDENTS_PREFIX, &north_20, hour, later).await,
            None
        );
        assert_eq!(
            traffic_key_shown(&s, INCIDENTS_PREFIX, &north_20).await,
            "tomtom:52.9500,10.5500"
        );
        // Gut 30 km: neu, am eigenen Rasterpunkt.
        let north_30 = at(53.25, 10.5594);
        let asked = traffic_due(&s, INCIDENTS_PREFIX, &north_30, hour, later)
            .await
            .expect("fällig");
        assert_eq!(incidents_key(&asked), "tomtom:53.2500,10.5500");
        // Bis der neue Stand da ist, zeigt die Anzeige noch den von zu Hause.
        assert_eq!(
            traffic_key_shown(&s, INCIDENTS_PREFIX, &north_30).await,
            "tomtom:52.9500,10.5500"
        );
        store::save(&s.pool, &incidents_key(&asked), later, &2)
            .await
            .unwrap();
        assert_eq!(
            traffic_key_shown(&s, INCIDENTS_PREFIX, &north_30).await,
            "tomtom:53.2500,10.5500"
        );
        // Zurück zu Hause gilt wieder der Stand von dort, solange er frisch ist.
        assert_eq!(
            traffic_key_shown(&s, INCIDENTS_PREFIX, &home).await,
            "tomtom:52.9500,10.5500"
        );
        assert_eq!(
            traffic_due(&s, INCIDENTS_PREFIX, &home, hour, later + TRAFFIC_RETRY_MS).await,
            None
        );
        // Weit weg gibt es nichts zu zeigen außer dem eigenen Rasterpunkt.
        assert_eq!(
            traffic_key_shown(&s, INCIDENTS_PREFIX, &at(54.0, 10.5594)).await,
            "tomtom:54.0000,10.5500"
        );
        // Andere Ebene, eigener Versuch.
        assert!(
            traffic_due(&s, SPEED_CAMERAS_PREFIX, &home, hour, later)
                .await
                .is_some()
        );
    }
}
