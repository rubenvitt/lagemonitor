//! Externe Datenquellen. Task 2 füllt `nina`, Task 3 `weather`.

pub mod nina;
pub mod weather;

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("Abruf fehlgeschlagen: {0}")]
    Http(String),
    #[error("Antwort nicht lesbar: {0}")]
    Parse(String),
}
