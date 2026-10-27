use std::sync::Arc;

use lagemonitor::app::{self, AppState};
use lagemonitor::config::BootConfig;
use lagemonitor::db;
use tokio::task::JoinSet;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let boot = Arc::new(BootConfig::from_env()?);
    let pool = db::connect(&boot).await?;
    let router = app::router(AppState {
        pool,
        boot: boot.clone(),
    });

    let mut servers = JoinSet::new();
    for addr in &boot.binds {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!(%addr, "lauscht");
        servers.spawn(axum::serve(listener, router.clone()).into_future());
    }
    // Endet ein Listener, endet das ganze Programm.
    if let Some(done) = servers.join_next().await {
        done??;
    }
    Ok(())
}
