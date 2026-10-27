# Lagemonitor v1 (Karte, DWD-Wetter, NINA) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Der Kiosk-Bildschirm im FüKw zeigt nach dem Einschalten ohne Zutun eine Karte am eingestellten Standort, das DWD-Wetter und die NINA-Warnungen für den Landkreis; Warnungen über der Schwelle erscheinen als Band.

**Architecture:** Ein Rust-Binary (axum) holt NINA- und Bright-Sky-Daten im Hintergrund, legt den letzten guten Stand in SQLite ab und liefert ihn mit Zeitstempel über `GET /api/monitor` aus. Das React-Frontend (antd 6, Lifeline-Designsystem aus Suchlage) ist per rust-embed eingebettet, fragt alle 30 s und entscheidet in einer reinen Funktion, was Platz bekommt. Karte offline aus einer MBTiles-Datei, sonst online über VersaTiles (gleiches Shortbread-Schema).

**Tech Stack:** Rust 1.99 (axum 0.8, sqlx 0.9 SQLite, reqwest 0.13 rustls, rust-embed 8, ts-rs 12), React 19 + Vite + antd 6.6.5 + maplibre-gl 6.13 + @tanstack/react-query 5, Docker Compose.

**Spec:** `docs/superpowers/specs/2026-10-27-lagemonitor-design.md` (Entwürfe: `docs/design/2026-10-27-zustaende.html`)

**Referenzprojekt (nur lesen):** `~/dev/personal/drk/suchlage` – Versionen, Theme, Kartenstil, Static-Serving, Dockerfile und Gate werden von dort übernommen.

## Umfang v1 (Nutzerentscheidung 27.10.2026)

Drin: Karte, DWD-Wetter (Bright Sky), NINA-Warnungen, Ruhe- und Warnzustand, Datenstand/„veraltet“, Betrieb im Container.
**Nicht** in v1: Verwaltung, Anmeldung (Pocket ID), Schaufenster, Lagemeldung, GPS-Standort, LLH. Suchlage später nur, wenn erreichbar. Daraus folgt: v1 lauscht **nur** auf Loopback und LAN, Fernzugang (Tunnel/Tailnet) bleibt aus, bis die Anmeldung existiert.

## Global Constraints

- Versionen exakt wie Suchlage: `rust-toolchain.toml` channel `1.99.0`, edition 2024; Node `26.7.0`, pnpm `11.10.0`; Frontend-Abhängigkeiten in den Versionen aus `suchlage/frontend/package.json`.
- Env nur Bootstrap (Grundsatz „keine Env-Konfiguration“, Suchlage ADR 0002): `LAGEMONITOR_DATA_DIR` (Standard `./data`), `LAGEMONITOR_BIND` (kommagetrennte Liste, Standard `127.0.0.1:8080`), `RUST_LOG`. Alles Fachliche steht in der Tabelle `settings`.
- Neuer Code: Bezeichner Englisch, Kommentare/Oberflächentexte Deutsch mit echten Umlauten. Aus Suchlage kopierte Dateien behalten ihre Namen und bekommen eine Herkunftszeile (`// Übernommen aus Suchlage <commit> (27.10.2026): <pfad>`).
- Keine Personendaten, keine echten Einsatzdaten in Fixtures. NINA-/Bright-Sky-Antworten sind öffentliche Daten und dürfen als Fixture mitgeschnitten werden.
- Designsystem „Instrumententafel“: Nacht Vorgabe, Radius 0, Haarlinien, Status immer auch als Wort/Zahl, Zahlen in JetBrains Mono, Schriften lokal (kein CDN).
- Upstream-Endpunkte: NINA `https://warnung.bund.de/api31`, Bright Sky `https://api.brightsky.dev`, Online-Kacheln wie in Suchlages `basemapStyle.ts` (VersaTiles), Radar DWD-WMS `https://maps.dwd.de/geoserver`.
- Jeder Commit läuft vorher durch `scripts/check.sh` (ab Task 1 Rust-Teil, ab Task 5 auch Frontend). Commits enden mit den Attributionszeilen der Session.

## Review Focus

1. **Start ohne Internet:** letzter Stand kommt aus SQLite, `stale: true`; leere DB → `data: null`, Oberfläche zeigt „Noch keine Daten“ statt Fehlerseite. → Test in Task 4 (`monitor_serves_snapshot_when_upstream_down`) und Task 6 (`shows_no_data_hint`).
2. **Upstream liefert 500, Timeout oder kaputtes JSON:** alter Stand bleibt, `error` gesetzt, Poller läuft weiter, kein Panic. → Task 4 `failed_fetch_keeps_previous_snapshot`.
3. **Warnung ohne Geometrie (GeoJSON 404/leer):** Warnung erscheint trotzdem, `geometry: null`. → Task 2 `warning_without_geojson_is_kept`.
4. **Abgelaufene Warnung im veralteten Stand:** `expires < now` erscheint weder im Band noch in der Liste. → Task 6 `drops_expired_warnings`.
5. **Mehrere Warnungen über der Schwelle:** Band zeigt die schwerste zuerst und „+n weitere“. → Task 6 `band_sorts_by_severity_and_counts_rest`.

---

## Vertrag (von Task 1–4 erzeugt, von Task 5–6 gelesen)

Rust-Typen mit `#[derive(Serialize, ts_rs::TS)] #[serde(rename_all = "camelCase")] #[ts(export, export_to = "../frontend/src/contract/")]` in `src/contract.rs`. `cargo test` schreibt die `.ts`-Dateien.

```rust
pub enum Severity { Unknown, Minor, Moderate, Severe, Extreme }        // Ord in dieser Reihenfolge, serde: "Unknown" … "Extreme"
pub enum Provider { Dwd, Mowas, Katwarn, Biwapp, Lhp, Police }          // serde: lowercase
pub struct Warning { id: String, provider: Provider, event: String, headline: String,
    description: String, instruction: Option<String>, severity: Severity, level_label: String,
    onset: Option<String>, expires: Option<String>, sender_name: Option<String>,
    geometry: Option<serde_json::Value> }                                // GeoJSON FeatureCollection; TS-Typ `unknown`
pub struct CurrentWeather { timestamp: String, temperature: Option<f64>, condition: Option<String>,
    icon: Option<String>, wind_speed: Option<f64>, wind_gust_speed: Option<f64>,
    wind_direction: Option<f64>, precipitation: Option<f64> }
pub struct HourlyWeather { timestamp: String, temperature: Option<f64>, icon: Option<String>, precipitation: Option<f64> }
pub struct Weather { current: CurrentWeather, hourly: Vec<HourlyWeather> }   // hourly: nächste 12 Stunden ab jetzt
pub struct Sourced<T> { data: Option<T>, fetched_at_ms: Option<i64>, stale: bool, error: Option<String> }
pub struct Location { lat: f64, lon: f64, label: String }
pub struct Region { ars: String, name: String }
pub struct MapInfo { offline: bool }
pub struct MonitorView { now_ms: i64, location: Location, region: Region, warn_threshold: Severity,
    weather: Sourced<Weather>, warnings: Sourced<Vec<Warning>>, map: MapInfo }
```

---

### Task 1: Backend-Gerüst, Einstellungen, Static-Serving

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `mise.toml`, `.cargo/config.toml` (nur falls Suchlage eine hat – übernehmen), `build.rs` (wie Suchlage: legt `frontend/dist/.gitkeep` an, damit rust-embed ohne Build kompiliert), `frontend/dist/.gitkeep`
- Create: `src/main.rs`, `src/lib.rs`, `src/config.rs`, `src/app.rs`, `src/error.rs`, `src/db.rs`, `src/settings.rs`, `src/static_files.rs`, `src/contract.rs`
- Create: `migrations/0001_init.sql`, `scripts/check.sh`
- Modify: `.gitignore` (`frontend/dist/*` + `!frontend/dist/.gitkeep`)

**Interfaces:**
- Produces: `BootConfig { data_dir: PathBuf, binds: Vec<SocketAddr> }` mit `from_lookup(impl Fn(&str)->Option<String>) -> Result<Self, BootError>`, `db_path() -> data_dir/lagemonitor.db`, `tiles_path() -> data_dir/karte.mbtiles`
- Produces: `db::connect(&BootConfig) -> Result<SqlitePool, AppError>` (legt Datei an, führt `sqlx::migrate!()` aus)
- Produces: `settings::Settings { location: Location, region: Region, warn_threshold: Severity }`, `settings::load(&SqlitePool) -> Result<Settings, AppError>` (fehlende Schlüssel → Standardwerte)
- Produces: `app::AppState { pool: SqlitePool, boot: Arc<BootConfig>, … }` (Task 4 ergänzt Felder), `app::router(AppState) -> Router` mit `/api/health` und SPA-Fallback
- Produces: in `contract.rs` die Typen `Severity`, `Location`, `Region` (Rest in Task 2–4)

Tabellen in `0001_init.sql`:
```sql
CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);           -- value = JSON
CREATE TABLE snapshots (source TEXT PRIMARY KEY, fetched_at_ms INTEGER NOT NULL, body TEXT NOT NULL);
```
Standardwerte (Schlüssel → JSON): `location` → `{"lat":52.9656,"lon":10.5594,"label":"Uelzen"}`, `region` → `{"ars":"033600000000","name":"Landkreis Uelzen"}`, `warn_threshold` → `"Severe"`.

- [ ] **Step 1: Gerüst anlegen.** `Cargo.toml` mit den Suchlage-Versionen von axum, axum-extra (nicht nötig – weglassen), tokio, sqlx, libsqlite3-sys bundled, serde, serde_json, thiserror, tracing, tracing-subscriber, chrono, reqwest (json, rustls), rust-embed, mime_guess, tower, tower-http (trace, catch-panic), ts-rs; dev: tempfile, http-body-util. `static_files.rs` aus Suchlage übernehmen (ohne `internal/`-Zweig). `scripts/check.sh` als schlanke Fassung von Suchlages Gate: `--rust` = fmt (CI: `--check`), `clippy --all-targets -D warnings`, `cargo test`, Vertrag-veraltet-Prüfung wie Suchlage; `--frontend` = `pnpm -C frontend lint typecheck test build` (Teil erst aktiv, wenn `frontend/package.json` existiert).
- [ ] **Step 2: Failing tests schreiben**
  - `config::tests::binds_default_to_loopback` – ohne Env: `binds == ["127.0.0.1:8080"]`, `data_dir == "./data"`
  - `config::tests::binds_accept_comma_list` – `"127.0.0.1:8080, 192.168.10.5:8080"` → zwei Adressen
  - `config::tests::invalid_bind_names_variable` – `err.var == "LAGEMONITOR_BIND"`
  - `settings::tests::defaults_when_table_empty` – frische DB → `location.label == "Uelzen"`, `region.ars == "033600000000"`, `warn_threshold == Severity::Severe`
  - `settings::tests::stored_value_wins` – `region` in DB gesetzt → wird geladen
  - `app::tests::health_ok` – `GET /api/health` → 200, `{"status":"ok"}`
  - `app::tests::unknown_api_is_json_404` – `GET /api/nope` → 404, kein HTML
  - Die `static_files::tests` aus Suchlage mitnehmen.
- [ ] **Step 3:** `cargo test` → FAIL (nicht implementiert).
- [ ] **Step 4: Implementieren.** `main.rs`: Tracing, `BootConfig::from_env()`, `db::connect`, je Adresse in `binds` ein `tokio::net::TcpListener` und `axum::serve` (alle mit demselben Router, per `futures::future::try_join_all` oder `JoinSet`). `Severity` mit `#[derive(PartialOrd, Ord)]` in der Reihenfolge des Vertrags.
- [ ] **Step 5:** `scripts/check.sh --rust` → grün.
- [ ] **Step 6: Commit** `feat: Backend-Gerüst mit Einstellungen und eingebetteter Oberfläche`

### Task 2: NINA-Quelle

**Files:**
- Create: `src/sources/mod.rs`, `src/sources/nina.rs`, `tests/fixtures/nina/` (mitgeschnittene Antworten)
- Modify: `src/contract.rs` (+ `Provider`, `Warning`), `src/lib.rs`

**Interfaces:**
- Consumes: `Severity`, `Region` aus Task 1
- Produces: `nina::fetch(http: &reqwest::Client, base: &Url, ars: &str) -> Result<Vec<Warning>, SourceError>`; reine Hilfen `nina::parse_dashboard(&[u8]) -> Result<Vec<String>, SourceError>` (liefert IDs), `nina::parse_warning(id: &str, cap: &[u8], geojson: Option<&[u8]>) -> Result<Warning, SourceError>`, `nina::provider_of(id: &str) -> Provider`, `nina::level_label(Provider, Severity) -> String`
- Produces: `sources::SourceError` (thiserror: `Http(String)`, `Parse(String)`)

Ablauf von `fetch`: `GET {base}/dashboard/{ars}.json` → IDs; je ID `GET {base}/warnings/{id}.json` (CAP, Felder aus `info[0]`: `event`, `headline`, `description`, `instruction`, `severity`, `onset`, `expires`, `senderName`) und `GET {base}/warnings/{id}.geojson`. GeoJSON-Fehler (404, Timeout, leer) → `geometry: None`, Warnung bleibt. Timeout pro Anfrage 10 s.

Zuordnungen:
- `provider_of`: Präfix `dwd` → Dwd, `mow` → Mowas, `kat` → Katwarn, `biw` → Biwapp, `lhp` → Lhp, `pol` → Police; sonst Mowas.
- `level_label`: Dwd: Minor „Stufe 1“, Moderate „Stufe 2“, Severe „Stufe 3“, Extreme „Stufe 4“, Unknown „Unbekannt“. Alle anderen: Minor „Gering“, Moderate „Mittel“, Severe „Hoch“, Extreme „Extrem“, Unknown „Unbekannt“.
- CAP-`severity` außerhalb der fünf Werte → `Unknown`.

- [ ] **Step 1: Fixtures mitschneiden.** Das Dashboard für `033600000000` ist oft leer. Ein Dashboard mit Einträgen finden: `GET https://warnung.bund.de/api31/dwd/mapData.json` und `/mowas/mapData.json` liefern aktuelle IDs; für eine davon CAP (`/warnings/{id}.json`) und GeoJSON speichern. Für ein Dashboard mit Einträgen Landkreis-ARS durchprobieren (`<5-stelliger Kreisschlüssel>0000000`) bis eines nicht `[]` ist, und speichern. Dazu ein leeres Dashboard. Ablage: `tests/fixtures/nina/{dashboard-leer,dashboard-voll,cap-dwd,cap-mowas,geojson-dwd}.json`. Stimmt das Dashboard-Format nicht mit der Annahme „Array von Objekten mit `id`“ überein, `parse_dashboard` an das echte Format anpassen und das im Commit vermerken.
- [ ] **Step 2: Failing tests**
  - `empty_dashboard_yields_no_ids` → `[]`
  - `dashboard_yields_ids` → IDs aus dem vollen Fixture, Anzahl = Einträge
  - `parses_dwd_cap` → `provider == Dwd`, `event`, `headline`, `severity`, `level_label == "Stufe 2"` (bei Moderate), `onset`/`expires` gesetzt, `geometry.is_some()`
  - `parses_mowas_cap_level_words` → `level_label` ist ein Wort aus der Liste
  - `warning_without_geojson_is_kept` → `parse_warning(id, cap, None)` ergibt `geometry: None`
  - `unknown_severity_maps_to_unknown`
  - `fetch_against_local_server` – axum-Testserver auf `127.0.0.1:0` liefert die Fixtures unter `/dashboard/…`, `/warnings/…`; GeoJSON-Route liefert 404 → `fetch` liefert die Warnung mit `geometry: None`
- [ ] **Step 3:** `cargo test nina` → FAIL.
- [ ] **Step 4:** Implementieren (serde-Structs nur für die benötigten Felder, `#[serde(default)]` großzügig).
- [ ] **Step 5:** `scripts/check.sh --rust` → grün.
- [ ] **Step 6: Commit** `feat: NINA-Warnungen je Regionalschlüssel`

### Task 3: Wetter-Quelle (Bright Sky)

**Files:**
- Create: `src/sources/weather.rs`, `tests/fixtures/brightsky/{current,hourly}.json`
- Modify: `src/contract.rs` (+ `CurrentWeather`, `HourlyWeather`, `Weather`)

**Interfaces:**
- Consumes: `Location` aus Task 1, `SourceError` aus Task 2 (Task 2 und 3 können parallel laufen; wer zuerst fertig ist, legt `sources/mod.rs` mit `SourceError` an, der andere übernimmt)
- Produces: `weather::fetch(http, base: &Url, loc: &Location, now: DateTime<Utc>) -> Result<Weather, SourceError>`; rein: `weather::parse_current(&[u8]) -> Result<CurrentWeather, SourceError>`, `weather::parse_hourly(&[u8], now) -> Result<Vec<HourlyWeather>, SourceError>`

Anfragen: `GET {base}/current_weather?lat=..&lon=..` (Felder: `timestamp`, `temperature`, `condition`, `icon`, `wind_speed_10`, `wind_gust_speed_10`, `wind_direction_10`, `precipitation_10`) und `GET {base}/weather?lat=..&lon=..&date=<now>&last_date=<now+13h>` (je Eintrag `timestamp`, `temperature`, `icon`, `precipitation`). `hourly` = Einträge mit `timestamp > now`, höchstens 12.

- [ ] **Step 1:** Fixtures für Uelzen mitschneiden (`lat=52.9656&lon=10.5594`).
- [ ] **Step 2: Failing tests**
  - `parses_current` – Werte aus dem Fixture (`temperature`, `windGustSpeed` aus `wind_gust_speed_10`)
  - `hourly_starts_after_now_and_caps_at_12`
  - `missing_fields_are_none` – Eintrag mit `null`-Werten parst ohne Fehler
  - `fetch_against_local_server` wie in Task 2
- [ ] **Step 3:** `cargo test weather` → FAIL.
- [ ] **Step 4:** Implementieren.
- [ ] **Step 5:** `scripts/check.sh --rust` → grün.
- [ ] **Step 6: Commit** `feat: DWD-Wetter über Bright Sky`

### Task 4: Abruf im Hintergrund, Datenstand, `/api/monitor`, Kacheln

**Files:**
- Create: `src/store.rs`, `src/poller.rs`, `src/web/mod.rs`, `src/web/monitor.rs`, `src/web/tiles.rs`
- Modify: `src/app.rs`, `src/main.rs`, `src/contract.rs` (+ `Sourced`, `MapInfo`, `MonitorView`)

**Interfaces:**
- Consumes: `nina::fetch`, `weather::fetch`, `settings::load`, `BootConfig::tiles_path`
- Produces: `store::save(pool, source: &str, fetched_at_ms, &impl Serialize)`, `store::load<T: DeserializeOwned>(pool, source) -> Option<(i64, T)>`; Quellen-Schlüssel `"nina"`, `"weather"`
- Produces: `poller::spawn(state: AppState)` und testbar `poller::run_once(state, source, fetcher, now_ms)`; Laufzeitfehler je Quelle in `AppState.errors: Arc<RwLock<HashMap<&'static str, String>>>`
- Produces: `GET /api/monitor -> MonitorView`, `GET /api/tiles/{z}/{x}/{y}` (aus `karte.mbtiles`, 404 ohne Datei/Kachel)

Regeln:
- Intervalle: NINA 60 s, Wetter 10 min; erster Lauf direkt beim Start. Basis-URLs als Konstanten in `poller.rs` (Tests übergeben eigene).
- Erfolg → `store::save`, Fehler-Eintrag löschen. Fehler → alter Stand bleibt, Fehlertext setzen, `tracing::warn!`, weiterlaufen.
- `stale`: NINA älter als 5 min, Wetter älter als 45 min, oder kein Stand.
- `MapInfo.offline` = `tiles_path()` existiert.
- Kacheln: MBTiles `tiles(zoom_level, tile_column, tile_row, tile_data)`, TMS: `tile_row = 2^z - 1 - y`. Antwort mit `Content-Type: application/x-protobuf`, `Content-Encoding: gzip` (Shortbread aus `karten-build` ist gzip), `Cache-Control: public, max-age=86400`. Die MBTiles-Datei mit einem eigenen read-only `SqlitePool` öffnen, wenn vorhanden.

- [ ] **Step 1: Failing tests**
  - `store_roundtrip`
  - `monitor_serves_snapshot_when_upstream_down` – Snapshot in DB von vor 10 min, Fetcher schlägt fehl → `warnings.data` = gespeicherter Stand, `stale: true`, `error.is_some()`
  - `failed_fetch_keeps_previous_snapshot` – erst Erfolg, dann Fehler → Daten unverändert, Poller-Funktion gibt keinen Panic
  - `empty_db_yields_null_data` – `data: null`, `fetchedAtMs: null`, `stale: true`
  - `monitor_contains_settings` – `location.label == "Uelzen"`, `warnThreshold == "Severe"`
  - `tiles_flip_y` – Test-MBTiles (tempfile, eine Kachel z=1 col=0 row=1) → `GET /api/tiles/1/0/0` liefert sie mit gzip-Header
  - `tiles_404_without_file` und `map_offline_false_without_file`
- [ ] **Step 2:** `cargo test` → FAIL.
- [ ] **Step 3:** Implementieren; `main.rs` startet `poller::spawn`.
- [ ] **Step 4:** `scripts/check.sh --rust` → grün; Vertrag in `frontend/src/contract/` erzeugt und eingecheckt.
- [ ] **Step 5: Live-Probe:** `cargo run`, nach 15 s `curl -s 127.0.0.1:8080/api/monitor | head -c 600` zeigt `weather.data` mit Werten und `warnings.data` als Array.
- [ ] **Step 6: Commit** `feat: Datenstand im Hintergrund, /api/monitor und Offline-Kacheln`

### Task 5: Frontend-Gerüst mit Lifeline-Designsystem und Grundkarte

Kann parallel zu Task 2–4 laufen (braucht nur den Vertrag aus dem Plan; bis Task 4 die `.ts`-Dateien erzeugt, die Typen aus dem Abschnitt „Vertrag“ von Hand in `frontend/src/contract/` anlegen – Task 4 überschreibt sie).

**Files:**
- Create: `frontend/package.json`, `pnpm-lock.yaml`, `pnpm-workspace.yaml`, `tsconfig.json`, `vite.config.ts`, `vitest.setup.ts`, `eslint.config.js`, `index.html`
- Create (aus Suchlage übernehmen): `frontend/src/theme/*` (tokens, rollen.css, sprache.css, schriften.ts, ThemeModeProvider.tsx, dichte.ts, helligkeit.ts, rollen.guard.test.ts), `frontend/src/assets/fonts/*` inkl. `LIESMICH.md` und `lizenzen/`, `frontend/src/map/basemapStyle.ts` (+ Test), `frontend/src/map/maplibreWorker.ts`, FOUC-Skript in `index.html`
- Create: `frontend/src/main.tsx`, `frontend/src/api.ts`, `frontend/src/monitor/MonitorPage.tsx` (Platzhalter mit Karte)

**Interfaces:**
- Consumes: Vertragstypen aus `frontend/src/contract/`
- Produces: `api.ts`: `fetchMonitor(): Promise<MonitorView>`, `useMonitor()` (react-query, `refetchInterval: 30_000`, `refetchIntervalInBackground: true`, Fehler behalten letzte Daten)
- Produces: `basemapStyle(theme: KartenTheme, tiles: { offline: boolean }): StyleSpecification` – offline: Vector-Source `tiles: ['/api/tiles/{z}/{x}/{y}']`, sonst die VersaTiles-URL aus Suchlage; Glyphen/Sprite wie in Suchlage (sind sie online, bleibt das in v1 so und wird im Abschlussbericht als Offline-Lücke genannt)

Nur die Abhängigkeiten übernehmen, die gebraucht werden: `antd`, `react`, `react-dom`, `maplibre-gl`, `@tanstack/react-query`, `dayjs`; dev: vite, @vitejs/plugin-react, typescript, vitest, jsdom, @testing-library/react, @testing-library/jest-dom, eslint + Konfiguration wie Suchlage. Kein Router (eine Seite).

- [ ] **Step 1:** Übernahmen kopieren, Herkunftszeilen setzen, `pnpm install`.
- [ ] **Step 2: Failing test** `basemapStyle.test.ts`: `offline: true` → Source-`tiles` ist `['/api/tiles/{z}/{x}/{y}']`; `offline: false` → VersaTiles-URL.
- [ ] **Step 3:** Implementieren; `MonitorPage` zeigt eine Vollbild-Karte am `location` aus `useMonitor()`.
- [ ] **Step 4:** `scripts/check.sh` (komplett) → grün; `pnpm -C frontend build` füllt `frontend/dist`, `cargo build` bettet ein, `cargo run` + Browser auf `127.0.0.1:8080` zeigt die Karte.
- [ ] **Step 5: Commit** `feat: Oberfläche mit Lifeline-Designsystem und Grundkarte`

### Task 6: Monitor-Ansicht (Ruhe, Warnung, Datenstand)

**Files:**
- Create: `frontend/src/monitor/selectLayout.ts` (+ `.test.ts`), `Header.tsx`, `Footer.tsx`, `WarningBand.tsx`, `WeatherPanel.tsx`, `MapPanel.tsx`, `monitor.css`, Tests je Komponente
- Modify: `frontend/src/monitor/MonitorPage.tsx`

**Interfaces:**
- Consumes: `useMonitor()`, `basemapStyle`, Vertragstypen
- Produces: `selectLayout(view: MonitorView, nowMs: number): { mode: 'ruhe' | 'warnung'; band: Warning[]; listed: Warning[] }`

Regeln für `selectLayout`:
- Abgelaufene Warnungen (`expires` vor `nowMs`) fallen ganz weg.
- `band` = Warnungen mit `severity >= warnThreshold` (Reihenfolge Unknown < Minor < Moderate < Severe < Extreme; `Unknown` nie im Band), absteigend nach Schwere, bei Gleichstand frühestes `onset` zuerst. `listed` = der Rest, gleiche Sortierung.
- `mode = band.length > 0 ? 'warnung' : 'ruhe'`.

Gestaltung nach `docs/design/2026-10-27-zustaende.html` (Zustände A und B; C entfällt in v1):
- Kopf 44 px: Markenzelle, Wortmarke `LAGEMONITOR · <location.label>`, Statuspunkt mit Wort („RUHE · keine Warnungen“ / „<n> WARNUNG(EN) · <höchste levelLabel>“), Uhr `HH:mm:ss` sekündlich.
- Warnband (nur `mode === 'warnung'`): getönte Alarmfläche, Kürzel `<EVENT> · <levelLabel>`, `headline`, rechts `bis HH:mm · <Quelle>`; bei mehr als einer Band-Warnung „+n weitere“.
- Karte (links, 2,2fr): Standortmarke, Warngebiete aller sichtbaren Warnungen als Fläche (Band: Alarmfarbe, Liste: Achtungsfarbe, gestrichelte Kante), Radar als WMS-Rasterebene nur wenn `!weather.stale`. **Vorher Layernamen prüfen** (`GetCapabilities` unter `https://maps.dwd.de/geoserver/ows?service=WMS&request=GetCapabilities`, Kandidat `dwd:Niederschlagsradar`); gibt es keinen passenden, Radar weglassen und im Bericht melden.
- Wetter (rechts, 1fr): Temperatur groß (Mono), Zustand als Wort, Zeilen Wind/Böen/Niederschlag, Vorhersageleiste mit 4 Stunden (aus `hourly` jeder dritte Eintrag), darunter „Weitere Warnungen“ = `listed` (Wort + Stufe).
- Fuß 26 px: `DWD/NINA · Stand HH:mm`, „Karte offline“ / „Karte online“, bei `stale` hervorgehoben „Veraltet · Stand HH:mm“ bzw. „Noch keine Daten“.
- Bright-Sky-`condition`/`icon` → deutsches Wort über eine feste Tabelle (dry „trocken“, fog „Nebel“, rain „Regen“, sleet „Schneeregen“, snow „Schnee“, hail „Hagel“, thunderstorm „Gewitter“; icon `clear-day/night` „klar“, `partly-cloudy-*` „heiter“, `cloudy` „bedeckt“, `wind` „windig“). Unbekannt → Rohwert.

- [ ] **Step 1: Failing tests**
  - `selectLayout.test.ts`: `drops_expired_warnings`, `band_sorts_by_severity_and_counts_rest` (zwei Severe + ein Extreme → Extreme zuerst), `unknown_never_in_band`, `threshold_moderate_includes_moderate`, `ruhe_without_warnings`
  - `WarningBand.test.tsx`: zeigt `levelLabel` als Text (zweiter Kanal), „+1 weitere“ bei zwei Band-Warnungen
  - `Footer.test.tsx`: `shows_no_data_hint` (data null → „Noch keine Daten“), `shows_stale` (stale → „Veraltet“)
  - `WeatherPanel.test.tsx`: übersetzt `condition` „rain“ → „Regen“
- [ ] **Step 2:** `pnpm -C frontend test` → FAIL.
- [ ] **Step 3:** Implementieren.
- [ ] **Step 4:** `scripts/check.sh` → grün. Sichtprüfung: `cargo run` mit gebautem Frontend, Screenshot bei 1920×1080 im Ruhezustand; Warnzustand mit einem temporär auf `Minor` gesetzten Schwellenwert (`sqlite3 data/lagemonitor.db "INSERT OR REPLACE INTO settings VALUES('warn_threshold','\"Minor\"')"`) und einem ARS mit aktiver Warnung prüfen, danach zurücksetzen.
- [ ] **Step 5: Commit** `feat: Monitor-Ansicht mit Warnband, Wetter und Datenstand`

### Task 7: Betrieb auf dem FüKw-Server

**Files:**
- Create: `deploy/Dockerfile` (nach Suchlage: node → rust → distroless, `ENV LAGEMONITOR_DATA_DIR=/data`), `deploy/compose.yaml`, `deploy/README.md`
- Modify: `deploy/kiosk/README.md` (Schritt 3: `KIOSK_URL=http://127.0.0.1:8080`), `README.md` (Installation, Nutzung, Einstellungen per `sqlite3` bis zur Verwaltung), `mise.toml` (Tasks `check`, `dev`, `image`)

`compose.yaml`: Dienst `lagemonitor`, `build: {context: .., dockerfile: deploy/Dockerfile}`, `network_mode: host` (damit `LAGEMONITOR_BIND` echte Host-Adressen bindet), `environment: LAGEMONITOR_BIND=127.0.0.1:8080,${LAN_IP}:8080`, `volumes: ./data:/data`, `restart: unless-stopped`. `deploy/README.md` beschreibt: Docker installieren, Repo klonen (Deploy-Key), `LAN_IP` in `deploy/.env` setzen, `docker compose -f deploy/compose.yaml up -d --build`, Kartenpaket: `make tiles AREA=germany` in `lifeline-hub/karten-build`, Datei nach `deploy/data/karte.mbtiles`, Dienst neu starten. Sicherheitsabschnitt: nie `0.0.0.0` binden; Uplink-Firewall (ufw: eingehend deny auf LTE/Sat-Interfaces, v4 und v6) – konkrete Interfacenamen folgen aus LGM-3.

- [ ] **Step 1:** Dateien schreiben.
- [ ] **Step 2:** `docker build -f deploy/Dockerfile -t lagemonitor:v1 .` → baut (falls lokal Docker verfügbar; sonst im Bericht vermerken).
- [ ] **Step 3:** Container lokal starten (`-e LAGEMONITOR_BIND=127.0.0.1:8080`, ohne host-Netz auf macOS: `-p 127.0.0.1:8080:8080` mit `LAGEMONITOR_BIND=0.0.0.0:8080` nur für diesen Test), `curl -s 127.0.0.1:8080/api/health` → `{"status":"ok"}`.
- [ ] **Step 4:** `scripts/check.sh` → grün.
- [ ] **Step 5: Commit** `feat: Betrieb per Docker Compose auf dem FüKw-Server`

---

## Abhängigkeiten

Task 1 → (Task 2 ∥ Task 3) → Task 4 → Task 6 → Task 7; Task 5 ∥ Task 2–4 (nach Task 1). Task 6 braucht Task 4 und 5.
