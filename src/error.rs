//! Ein Fehlertyp für alle Handler. Antwort immer `{"error": "<deutscher Satz>"}`.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Nicht gefunden")]
    NotFound,
    /// Details nur ins Log.
    #[error("Interner Serverfehler")]
    Internal(String),
}

impl AppError {
    pub fn status(&self) -> StatusCode {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if let AppError::Internal(detail) = &self {
            tracing::error!(%detail, "interner Fehler");
        }
        (
            self.status(),
            Json(serde_json::json!({ "error": self.to_string() })),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_detail_never_reaches_the_client() {
        let err = AppError::Internal("geheimes Detail".into());
        assert_eq!(err.to_string(), "Interner Serverfehler");
        assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
