//! Anmeldung über Pocket ID (OIDC, Authorization Code mit PKCE) für die Listener aus
//! `LAGEMONITOR_AUTH_BIND`. Der Kiosk und das Fahrzeugnetz (`LAGEMONITOR_BIND`) sehen davon nichts.
//!
//! Gruppen und Name kommen vom Userinfo-Endpunkt, abgefragt über TLS mit dem frischen Access-Token;
//! ein ID-Token muss deshalb nicht geprüft werden. Die Sitzung hält 14 Tage (Spec 4.3) und wird
//! lokal geprüft: Pocket ID braucht es nur beim Anmelden. Ein Gruppenentzug wirkt darum erst, wenn
//! die Sitzung abläuft.

use axum::extract::{Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::app::AppState;
use crate::config::OidcConfig;

pub const SESSION_COOKIE: &str = "lagemonitor_sitzung";
/// Bindet eine angefangene Anmeldung an den Browser, der sie begonnen hat (gegen Login-CSRF).
pub const STATE_COOKIE: &str = "lagemonitor_anmeldung";
const SESSION_MS: i64 = 14 * 24 * 3_600_000;
/// Ein Rechner mit Link bleibt so lange angemeldet, wie Browser ein Cookie höchstens halten (400 Tage);
/// vorher endet die Sitzung nur, wenn die Suite den Link widerruft.
const LINK_SESSION_MS: i64 = 400 * 24 * 3_600_000;
const LOGIN_MS: i64 = 10 * 60_000;
const SCOPES: &str = "openid profile groups";

/// Endpunkte aus `/.well-known/openid-configuration`; einmal geholt, dann behalten.
#[derive(Debug, Clone, Deserialize)]
pub struct Discovery {
    authorization_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct UserInfo {
    sub: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    preferred_username: Option<String>,
    #[serde(default)]
    groups: Vec<String>,
}

#[derive(Deserialize)]
pub struct LoginQuery {
    next: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("Zufallsquelle des Betriebssystems");
    URL_SAFE_NO_PAD.encode(bytes)
}

fn sha256_hex(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// Nur ein Pfad auf diesem Server, nie eine fremde Adresse (`//host`, `/\host`).
fn safe_next(next: Option<&str>) -> String {
    match next {
        Some(n) if n.starts_with('/') && !n.starts_with("//") && !n.starts_with("/\\") => {
            n.to_string()
        }
        _ => "/".into(),
    }
}

pub fn cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v)
}

fn set_cookie(oidc: &OidcConfig, name: &str, value: &str, max_age_s: i64) -> HeaderValue {
    let secure = if oidc.secure_cookies() {
        "; Secure"
    } else {
        ""
    };
    let path = match oidc.base_path() {
        "" => "/",
        base => base,
    };
    HeaderValue::from_str(&format!(
        "{name}={value}; Path={path}; HttpOnly; SameSite=Lax; Max-Age={max_age_s}{secure}"
    ))
    .expect("Cookie aus sicheren Zeichen")
}

fn oidc(state: &AppState) -> &OidcConfig {
    state
        .boot
        .oidc
        .as_ref()
        .expect("Anmelde-Router nur mit OIDC-Konfiguration")
}

fn page(status: StatusCode, text: &str) -> Response {
    let html = format!(
        "<!doctype html><html lang=\"de\"><meta charset=\"utf-8\"><title>Lagemonitor</title>\
         <body style=\"font-family:sans-serif;background:#111;color:#eee;padding:2rem\">\
         <h1>Lagemonitor</h1><p>{text}</p><p><a style=\"color:#9cf\" href=\"login\">Erneut anmelden</a></p></body></html>"
    );
    (
        status,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

async fn discovery(state: &AppState) -> Result<Discovery, String> {
    if let Some(d) = state.discovery.read().await.clone() {
        return Ok(d);
    }
    let issuer = oidc(state).issuer.as_str().trim_end_matches('/');
    let url = format!("{issuer}/.well-known/openid-configuration");
    let d: Discovery = state
        .http
        .get(&url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Discovery {url}: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Discovery {url}: {e}"))?;
    *state.discovery.write().await = Some(d.clone());
    Ok(d)
}

/// Gibt es zum Cookie eine laufende Sitzung? Eine Sitzung aus einem Link gilt nur, solange die Suite
/// den Link noch schickt (settings `public_links`, jede Minute abgeholt).
async fn session_valid(state: &AppState, headers: &HeaderMap) -> bool {
    let Some(token) = cookie(headers, SESSION_COOKIE) else {
        return false;
    };
    let row: Option<(Option<String>,)> = match sqlx::query_as(
        "SELECT link_id FROM sessions WHERE token_hash = ? AND expires_at_ms > ?",
    )
    .bind(sha256_hex(token))
    .bind(now_ms())
    .fetch_optional(&state.pool)
    .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::error!(error = %e, "Sitzung nicht prüfbar");
            return false;
        }
    };
    match row {
        None => false,
        Some((None,)) => true,
        Some((Some(link_id),)) => crate::settings::load(&state.pool)
            .await
            .map(|s| s.public_links.iter().any(|l| l.id == link_id))
            .unwrap_or(false),
    }
}

/// Legt eine Sitzung an und gibt den Schlüssel fürs Cookie zurück.
async fn create_session(
    state: &AppState,
    subject: &str,
    name: &str,
    link_id: Option<&str>,
    lifetime_ms: i64,
) -> Result<String, sqlx::Error> {
    let token = random_token();
    let now = now_ms();
    sqlx::query("DELETE FROM sessions WHERE expires_at_ms <= ?")
        .bind(now)
        .execute(&state.pool)
        .await?;
    sqlx::query(
        "INSERT INTO sessions (token_hash, subject, name, expires_at_ms, link_id) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(sha256_hex(&token))
    .bind(subject)
    .bind(name)
    .bind(now + lifetime_ms)
    .bind(link_id)
    .execute(&state.pool)
    .await?;
    Ok(token)
}

/// Riegel vor allem außer `/api/health` und `/auth/*`. Die Oberfläche wird zur Anmeldung
/// umgeleitet, die API antwortet 401 (die Oberfläche lädt dann neu).
pub async fn require(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if path == "/api/health" || session_valid(&state, request.headers()).await {
        return next.run(request).await;
    }
    if path.starts_with("/api/") {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({ "error": "Anmeldung nötig" })),
        )
            .into_response();
    }
    let target = request
        .uri()
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/");
    let mut login = reqwest::Url::parse("http://x/auth/login").expect("feste Adresse");
    login.query_pairs_mut().append_pair("next", target);
    Redirect::to(&format!(
        "{}/auth/login?{}",
        oidc(&state).base_path(),
        login.query().unwrap_or_default()
    ))
    .into_response()
}

pub async fn login(State(state): State<AppState>, Query(q): Query<LoginQuery>) -> Response {
    let d = match discovery(&state).await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(error = %e, "Pocket ID nicht erreichbar");
            return page(
                StatusCode::BAD_GATEWAY,
                "Die Anmeldung (Pocket ID) ist gerade nicht erreichbar. Bitte gleich noch einmal versuchen.",
            );
        }
    };
    let oidc = oidc(&state);
    let login_state = random_token();
    let verifier = random_token();
    let now = now_ms();
    let stored = async {
        sqlx::query("DELETE FROM logins WHERE created_at_ms < ?")
            .bind(now - LOGIN_MS)
            .execute(&state.pool)
            .await?;
        sqlx::query("INSERT INTO logins (state, verifier, next, created_at_ms) VALUES (?, ?, ?, ?)")
            .bind(&login_state)
            .bind(&verifier)
            .bind(safe_next(q.next.as_deref()))
            .bind(now)
            .execute(&state.pool)
            .await
    }
    .await;
    if let Err(e) = stored {
        return crate::error::AppError::from(e).into_response();
    }
    let Ok(mut url) = reqwest::Url::parse(&d.authorization_endpoint) else {
        return page(
            StatusCode::BAD_GATEWAY,
            "Pocket ID meldet eine ungültige Adresse.",
        );
    };
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &oidc.client_id)
        .append_pair("redirect_uri", &oidc.redirect_uri())
        .append_pair("scope", SCOPES)
        .append_pair("state", &login_state)
        .append_pair("code_challenge", &pkce_challenge(&verifier))
        .append_pair("code_challenge_method", "S256");
    let mut res = Redirect::to(url.as_str()).into_response();
    res.headers_mut().append(
        header::SET_COOKIE,
        set_cookie(oidc, STATE_COOKIE, &login_state, LOGIN_MS / 1000),
    );
    res
}

pub async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Response {
    if let Some(error) = q.error {
        tracing::info!(%error, "Anmeldung abgebrochen");
        return page(StatusCode::FORBIDDEN, "Die Anmeldung wurde abgebrochen.");
    }
    let (Some(code), Some(login_state)) = (q.code, q.state) else {
        return page(
            StatusCode::BAD_REQUEST,
            "Unvollständige Antwort der Anmeldung.",
        );
    };
    if cookie(&headers, STATE_COOKIE) != Some(login_state.as_str()) {
        return page(
            StatusCode::BAD_REQUEST,
            "Diese Anmeldung wurde in einem anderen Fenster begonnen oder ist abgelaufen.",
        );
    }
    let row: Option<(String, String)> = match sqlx::query_as(
        "DELETE FROM logins WHERE state = ? AND created_at_ms > ? RETURNING verifier, next",
    )
    .bind(&login_state)
    .bind(now_ms() - LOGIN_MS)
    .fetch_optional(&state.pool)
    .await
    {
        Ok(row) => row,
        Err(e) => return crate::error::AppError::from(e).into_response(),
    };
    let Some((verifier, next)) = row else {
        return page(StatusCode::BAD_REQUEST, "Die Anmeldung ist abgelaufen.");
    };
    let user = match exchange(&state, &code, &verifier).await {
        Ok(user) => user,
        Err(e) => {
            tracing::warn!(error = %e, "Anmeldung gescheitert");
            return page(
                StatusCode::BAD_GATEWAY,
                "Die Anmeldung bei Pocket ID hat nicht geklappt.",
            );
        }
    };
    let oidc = oidc(&state);
    if !user.groups.iter().any(|g| oidc.groups.contains(g)) {
        tracing::info!(sub = %user.sub, "Anmeldung ohne passende Gruppe");
        return page(
            StatusCode::FORBIDDEN,
            "Dein Konto darf den Lagemonitor nicht sehen. Frag in der Verwaltung nach der Gruppe für den Lagemonitor.",
        );
    }
    let name = user
        .name
        .or(user.preferred_username)
        .unwrap_or_else(|| user.sub.clone());
    let token = match create_session(&state, &user.sub, &name, None, SESSION_MS).await {
        Ok(token) => token,
        Err(e) => return crate::error::AppError::from(e).into_response(),
    };
    tracing::info!(sub = %user.sub, %name, "angemeldet");
    let mut res = Redirect::to(&format!("{}{next}", oidc.base_path())).into_response();
    let h = res.headers_mut();
    h.append(
        header::SET_COOKIE,
        set_cookie(oidc, SESSION_COOKIE, &token, SESSION_MS / 1000),
    );
    h.append(header::SET_COOKIE, set_cookie(oidc, STATE_COOKIE, "", 0));
    res
}

async fn exchange(state: &AppState, code: &str, verifier: &str) -> Result<UserInfo, String> {
    let d = discovery(state).await?;
    let oidc = oidc(state);
    let redirect_uri = oidc.redirect_uri();
    let token: TokenResponse = state
        .http
        .post(&d.token_endpoint)
        .basic_auth(&oidc.client_id, Some(&oidc.client_secret))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", verifier),
        ])
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Token: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Token: {e}"))?;
    state
        .http
        .get(&d.userinfo_endpoint)
        .bearer_auth(&token.access_token)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Userinfo: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Userinfo: {e}"))
}

/// Die Seite hinter einem Link aus der Suite (`…/auth/link#<Schlüssel>`). Der Schlüssel steht hinter
/// `#` und erreicht damit weder Proxy- noch Server-Logs: Die Seite liest ihn im Browser, tauscht ihn
/// per POST gegen eine Sitzung und nimmt ihn sofort aus der Adresszeile.
pub async fn link_page() -> Response {
    const HTML: &str = r#"<!doctype html><html lang="de"><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1"><meta name="referrer" content="no-referrer">
<title>Lagemonitor</title>
<body style="font-family:sans-serif;background:#111;color:#eee;padding:2rem">
<h1>Lagemonitor</h1><p id="t">Einen Moment, der Zugang wird eingerichtet …</p>
<script>
const key = location.hash.slice(1);
history.replaceState(null, "", location.pathname);
const say = (t) => { document.getElementById("t").textContent = t; };
if (!key) say("Dieser Link ist unvollständig. Bitte den ganzen Link aus der IUK-Suite öffnen.");
else fetch("link", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ key }) })
  .then((r) => r.ok ? location.replace("../") : say("Dieser Link gilt nicht (mehr). Bitte in der IUK-Suite einen neuen anlegen lassen."))
  .catch(() => say("Der Lagemonitor ist gerade nicht erreichbar. Bitte die Seite gleich neu laden."));
</script></body></html>"#;
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        HTML,
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct LinkBody {
    key: String,
}

/// Tauscht den Schlüssel eines Links gegen eine Sitzung. Der Vergleich läuft über den SHA-256, wie
/// die Suite ihn schickt; der Schlüssel selbst liegt nirgends.
pub async fn link_login(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<LinkBody>,
) -> Response {
    let hash = sha256_hex(body.key.trim());
    let links = match crate::settings::load(&state.pool).await {
        Ok(s) => s.public_links,
        Err(e) => return e.into_response(),
    };
    let Some(link) = links.into_iter().find(|l| l.hash == hash) else {
        tracing::info!("Link ohne Gültigkeit");
        return (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({ "error": "Dieser Link gilt nicht (mehr)." })),
        )
            .into_response();
    };
    let subject = format!("link:{}", link.id);
    let token = match create_session(
        &state,
        &subject,
        &link.label,
        Some(&link.id),
        LINK_SESSION_MS,
    )
    .await
    {
        Ok(token) => token,
        Err(e) => return crate::error::AppError::from(e).into_response(),
    };
    tracing::info!(link = %link.id, label = %link.label, "Rechner über Link angemeldet");
    let mut res = StatusCode::NO_CONTENT.into_response();
    res.headers_mut().append(
        header::SET_COOKIE,
        set_cookie(oidc(&state), SESSION_COOKIE, &token, LINK_SESSION_MS / 1000),
    );
    res
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(token) = cookie(&headers, SESSION_COOKIE)
        && let Err(e) = sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(sha256_hex(token))
            .execute(&state.pool)
            .await
    {
        return crate::error::AppError::from(e).into_response();
    }
    let mut res = page(StatusCode::OK, "Du bist abgemeldet.");
    res.headers_mut().append(
        header::SET_COOKIE,
        set_cookie(oidc(&state), SESSION_COOKIE, "", 0),
    );
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use axum::Router;
    use axum::body::Body;
    use axum::extract::Form;
    use axum::routing::{get, post};
    use tower::ServiceExt;

    use crate::config::BootConfig;

    /// Pocket ID als Doppel. Der Code ist zugleich das Access-Token; `nutzer` hat die Gruppe, alles
    /// andere nicht. Den zuletzt gesendeten PKCE-Verifier legt es in `verifier` ab.
    async fn fake_pocket_id(verifier: Arc<Mutex<Option<String>>>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let discovery = serde_json::json!({
            "issuer": base,
            "authorization_endpoint": format!("{base}/authorize"),
            "token_endpoint": format!("{base}/api/oidc/token"),
            "userinfo_endpoint": format!("{base}/api/oidc/userinfo"),
        });
        let app = Router::new()
            .route(
                "/.well-known/openid-configuration",
                get(move || async move { axum::Json(discovery) }),
            )
            .route(
                "/api/oidc/token",
                post(
                    move |h: HeaderMap, Form(f): Form<std::collections::HashMap<String, String>>| async move {
                        let basic = format!("Basic {}", base64::engine::general_purpose::STANDARD.encode("client:geheim"));
                        if h.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) != Some(&basic) {
                            return StatusCode::UNAUTHORIZED.into_response();
                        }
                        *verifier.lock().unwrap() = f.get("code_verifier").cloned();
                        axum::Json(serde_json::json!({ "access_token": f["code"], "token_type": "Bearer" }))
                            .into_response()
                    },
                ),
            )
            .route(
                "/api/oidc/userinfo",
                get(|h: HeaderMap| async move {
                    let groups = if h.get(header::AUTHORIZATION).unwrap() == "Bearer nutzer" {
                        vec!["lagemonitor-nutzer"]
                    } else {
                        vec!["andere"]
                    };
                    axum::Json(serde_json::json!({ "sub": "u1", "name": "Erika", "groups": groups }))
                }),
            );
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        base
    }

    async fn guarded(issuer: &str) -> Router {
        guarded_at(issuer, "http://lagebild.test").await.0
    }

    async fn guarded_at(issuer: &str, public_url: &str) -> (Router, AppState) {
        let issuer = issuer.to_string();
        let public_url = public_url.to_string();
        let boot = BootConfig::from_lookup(move |k| {
            match k {
                "LAGEMONITOR_AUTH_BIND" => Some("0.0.0.0:8080"),
                "LAGEMONITOR_OIDC_ISSUER" => Some(issuer.as_str()),
                "LAGEMONITOR_OIDC_CLIENT_ID" => Some("client"),
                "LAGEMONITOR_OIDC_CLIENT_SECRET" => Some("geheim"),
                "LAGEMONITOR_PUBLIC_URL" => Some(public_url.as_str()),
                _ => None,
            }
            .map(String::from)
        })
        .unwrap();
        let state = AppState::new(crate::db::test_pool().await, Arc::new(boot)).await;
        (crate::app::router_with_login(state.clone()), state)
    }

    async fn post_link(app: &Router, key: &str) -> Response {
        let req = axum::http::Request::post("/auth/link")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::json!({ "key": key }).to_string()))
            .unwrap();
        app.clone().oneshot(req).await.unwrap()
    }

    async fn set_links(state: &AppState, links: Vec<crate::settings::PublicLink>) {
        let mut s = crate::settings::load(&state.pool).await.unwrap();
        s.public_links = links;
        crate::settings::save(&state.pool, &s).await.unwrap();
    }

    #[tokio::test]
    async fn link_from_the_suite_opens_the_monitor_until_revoked() {
        let (app, state) = guarded_at("http://127.0.0.1:9", "http://lagebild.test").await;
        let key = "x".repeat(43);
        let link = crate::settings::PublicLink {
            id: "l1".into(),
            label: "Rechner Wache".into(),
            hash: sha256_hex(&key),
        };
        set_links(&state, vec![link]).await;
        let page = send(&app, "/auth/link", None).await;
        assert_eq!(page.status(), StatusCode::OK, "Seite ohne Anmeldung");

        assert_eq!(
            post_link(&app, "falsch").await.status(),
            StatusCode::FORBIDDEN
        );
        let res = post_link(&app, &key).await;
        assert_eq!(res.status(), StatusCode::NO_CONTENT);
        let cookie = format!(
            "{SESSION_COOKIE}={}",
            set_cookie_value(&res, SESSION_COOKIE)
        );
        assert_eq!(
            send(&app, "/api/monitor", Some(&cookie)).await.status(),
            StatusCode::OK
        );

        set_links(&state, vec![]).await;
        assert_eq!(
            send(&app, "/api/monitor", Some(&cookie)).await.status(),
            StatusCode::UNAUTHORIZED,
            "widerrufen in der Suite"
        );
    }

    #[tokio::test]
    async fn below_a_path_redirects_and_cookies_keep_the_prefix() {
        let verifier = Arc::default();
        let issuer = fake_pocket_id(Arc::clone(&verifier)).await;
        let (app, _) = guarded_at(&issuer, "https://lagemonitor.test/monitor/").await;
        let res = send(&app, "/", None).await;
        assert_eq!(location(&res).path(), "/monitor/auth/login");
        let res = send(&app, "/auth/login?next=/", None).await;
        let state = param(&location(&res), "state");
        assert_eq!(
            param(&location(&res), "redirect_uri"),
            "https://lagemonitor.test/monitor/auth/callback"
        );
        let set = res.headers()[header::SET_COOKIE].to_str().unwrap();
        assert!(
            set.contains("Path=/monitor;") && set.contains("Secure"),
            "{set}"
        );
        let res = send(
            &app,
            &format!("/auth/callback?code=nutzer&state={state}"),
            Some(&format!("{STATE_COOKIE}={state}")),
        )
        .await;
        assert_eq!(res.headers()[header::LOCATION], "/monitor/");
    }

    async fn send(app: &Router, uri: &str, cookie: Option<&str>) -> Response {
        let mut req = axum::http::Request::get(uri);
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        app.clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    fn location(res: &Response) -> reqwest::Url {
        let loc = res.headers()[header::LOCATION].to_str().unwrap();
        reqwest::Url::parse("http://lagebild.test")
            .unwrap()
            .join(loc)
            .unwrap()
    }

    fn param(url: &reqwest::Url, name: &str) -> String {
        url.query_pairs()
            .find(|(k, _)| k == name)
            .unwrap()
            .1
            .into_owned()
    }

    fn set_cookie_value(res: &Response, name: &str) -> String {
        res.headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|v| v.to_str().unwrap())
            .find_map(|v| v.strip_prefix(&format!("{name}=")))
            .and_then(|v| v.split(';').next())
            .unwrap()
            .to_string()
    }

    /// Bis zum Rücksprung von Pocket ID: liefert Antwort des Callbacks und den gesendeten Verifier.
    async fn sign_in(
        app: &Router,
        code: &str,
        verifier: &Arc<Mutex<Option<String>>>,
    ) -> (Response, String) {
        let res = send(app, "/?ansicht=karte", None).await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        let login = location(&res);
        assert_eq!(login.path(), "/auth/login");
        let res = send(
            app,
            &format!("/auth/login?{}", login.query().unwrap()),
            None,
        )
        .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        let authorize = location(&res);
        assert!(authorize.path().ends_with("/authorize"));
        assert_eq!(
            param(&authorize, "redirect_uri"),
            "http://lagebild.test/auth/callback"
        );
        assert_eq!(param(&authorize, "code_challenge_method"), "S256");
        let challenge = param(&authorize, "code_challenge");
        let state = param(&authorize, "state");
        assert_eq!(set_cookie_value(&res, STATE_COOKIE), state);
        let res = send(
            app,
            &format!("/auth/callback?code={code}&state={state}"),
            Some(&format!("{STATE_COOKIE}={state}")),
        )
        .await;
        let sent = verifier.lock().unwrap().clone().unwrap();
        assert_eq!(pkce_challenge(&sent), challenge, "PKCE passt");
        (res, state)
    }

    #[tokio::test]
    async fn without_session_ui_redirects_and_api_refuses() {
        let app = guarded("http://127.0.0.1:9").await;
        let res = send(&app, "/", None).await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(param(&location(&res), "next"), "/");
        assert_eq!(
            send(&app, "/api/monitor", None).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send(&app, "/api/tiles/1/0/0", None).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send(&app, "/api/monitor", Some("lagemonitor_sitzung=erfunden"))
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send(&app, "/api/health", None).await.status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn member_signs_in_and_sees_the_monitor() {
        let verifier = Arc::default();
        let app = guarded(&fake_pocket_id(Arc::clone(&verifier)).await).await;
        let (res, _) = sign_in(&app, "nutzer", &verifier).await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        assert_eq!(location(&res).path(), "/");
        assert_eq!(location(&res).query(), Some("ansicht=karte"));
        let session = set_cookie_value(&res, SESSION_COOKIE);
        let cookie = format!("{SESSION_COOKIE}={session}");
        assert_eq!(
            send(&app, "/api/monitor", Some(&cookie)).await.status(),
            StatusCode::OK
        );

        let res = send(&app, "/auth/logout", Some(&cookie)).await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            send(&app, "/api/monitor", Some(&cookie)).await.status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn account_without_group_is_turned_away() {
        let verifier = Arc::default();
        let app = guarded(&fake_pocket_id(Arc::clone(&verifier)).await).await;
        let (res, _) = sign_in(&app, "fremd", &verifier).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        assert!(res.headers().get(header::SET_COOKIE).is_none());
    }

    #[tokio::test]
    async fn callback_from_another_browser_is_refused_and_state_is_single_use() {
        let verifier = Arc::default();
        let app = guarded(&fake_pocket_id(Arc::clone(&verifier)).await).await;
        let res = send(&app, "/auth/login", None).await;
        let state = param(&location(&res), "state");
        let res = send(
            &app,
            &format!("/auth/callback?code=nutzer&state={state}"),
            None,
        )
        .await;
        assert_eq!(res.status(), StatusCode::BAD_REQUEST, "ohne Cookie");
        let cookie = format!("{STATE_COOKIE}={state}");
        let uri = format!("/auth/callback?code=nutzer&state={state}");
        assert_eq!(
            send(&app, &uri, Some(&cookie)).await.status(),
            StatusCode::SEE_OTHER
        );
        assert_eq!(
            send(&app, &uri, Some(&cookie)).await.status(),
            StatusCode::BAD_REQUEST,
            "nur einmal"
        );
    }

    #[test]
    fn next_stays_on_this_server() {
        assert_eq!(safe_next(Some("/?x=1")), "/?x=1");
        assert_eq!(safe_next(Some("//evil.example")), "/");
        assert_eq!(safe_next(Some("/\\evil.example")), "/");
        assert_eq!(safe_next(Some("https://evil.example")), "/");
        assert_eq!(safe_next(None), "/");
    }

    #[test]
    fn pkce_matches_rfc_7636_example() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn cookie_is_found_among_others() {
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            "a=1; lagemonitor_sitzung=abc; b=2".parse().unwrap(),
        );
        assert_eq!(cookie(&h, SESSION_COOKIE), Some("abc"));
        assert_eq!(cookie(&h, STATE_COOKIE), None);
    }
}
