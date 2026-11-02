use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use lagemonitor::app::{self, AppState};
use lagemonitor::config::BootConfig;
use lagemonitor::db;
use lagemonitor::poller;
use tokio::net::TcpListener;
use tokio::task::JoinSet;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let boot = Arc::new(BootConfig::from_env()?);
    let pool = db::connect(&boot).await?;
    let state = AppState::new(pool, boot.clone()).await;
    // Erst die Konfiguration, dann die Quellen: Der erste Wetterabruf soll schon den Ort der Suite
    // kennen. Ohne Netz nicht lange warten, der letzte Stand liegt in `settings`.
    let _ = tokio::time::timeout(CONFIG_AT_BOOT, poller::poll_config(&state)).await;
    poller::spawn(state.clone());
    let router = app::router(state.clone());

    let mut servers = JoinSet::new();
    // Loopback zuerst und zwingend: daran hängt der Kiosk. Eine LAN-Adresse fehlt beim Booten
    // womöglich noch (Kabel ab, DHCP spät) – dann im Hintergrund weiter versuchen, nicht abbrechen.
    for addr in boot.binds.iter().filter(|a| a.ip().is_loopback()) {
        let listener = TcpListener::bind(addr).await?;
        tracing::info!(%addr, "lauscht");
        servers.spawn(axum::serve(listener, router.clone()).into_future());
    }
    for &addr in boot.binds.iter().filter(|a| !a.ip().is_loopback()) {
        let router = router.clone();
        servers.spawn(
            async move { axum::serve(bind_with_retry(addr, LAN_RETRY).await, router).await },
        );
    }
    // Mit Anmeldung (Pocket ID) darf auch `0.0.0.0` sein: im Container hinter Traefik.
    if !boot.auth_binds.is_empty() {
        let guarded = app::router_with_login(state);
        for addr in &boot.auth_binds {
            let listener = TcpListener::bind(addr).await?;
            tracing::info!(%addr, "lauscht, nur mit Anmeldung");
            servers.spawn(axum::serve(listener, guarded.clone()).into_future());
        }
    }
    // Endet ein Listener, endet das ganze Programm.
    if let Some(done) = servers.join_next().await {
        done??;
    }
    Ok(())
}

const LAN_RETRY: Duration = Duration::from_secs(10);
const CONFIG_AT_BOOT: Duration = Duration::from_secs(3);

async fn bind_with_retry(addr: SocketAddr, every: Duration) -> TcpListener {
    loop {
        match TcpListener::bind(addr).await {
            Ok(listener) => {
                tracing::info!(%addr, "lauscht");
                return listener;
            }
            Err(e) => {
                tracing::warn!(%addr, error = %e, retry_s = every.as_secs(), "Adresse nicht verfügbar, neuer Versuch");
                tokio::time::sleep(every).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lan_bind_waits_until_address_is_free() {
        let occupier = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = occupier.local_addr().unwrap();
        let pending = tokio::spawn(bind_with_retry(addr, Duration::from_millis(10)));
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !pending.is_finished(),
            "darf erst binden, wenn die Adresse frei ist"
        );
        drop(occupier);
        let listener = tokio::time::timeout(Duration::from_secs(2), pending)
            .await
            .expect("bindet nach Freigabe")
            .unwrap();
        assert_eq!(listener.local_addr().unwrap(), addr);
    }
}
