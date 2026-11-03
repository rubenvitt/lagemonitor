//! HTTP-Client für alle Abrufe nach außen.

use std::sync::Once;

/// Ein Client mit rustls über ring. reqwest bringt ohne aws-lc keinen Anbieter mit; ohne
/// `install_default` bräche jeder Client beim Bau ab. Hier und nur hier wird er gesetzt.
pub fn client() -> reqwest::Client {
    static ANBIETER: Once = Once::new();
    ANBIETER.call_once(|| {
        // Hat schon jemand einen Anbieter gesetzt, gilt der.
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
    reqwest::Client::new()
}
