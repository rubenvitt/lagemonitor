//! `GET /api/tiles/{z}/{x}/{y}` aus `karte.mbtiles` (Shortbread-Vektorkacheln, gzip).

use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};

use crate::app::AppState;
use crate::error::AppError;

pub async fn get(
    State(state): State<AppState>,
    Path((z, x, y)): Path<(u8, u32, u32)>,
) -> Result<Response, AppError> {
    let pool = state.tiles.as_ref().ok_or(AppError::NotFound)?;
    // MBTiles zählt Zeilen von unten (TMS), die Karte von oben.
    let Some(row) = 1u32
        .checked_shl(z.into())
        .and_then(|n| n.checked_sub(1 + y))
    else {
        return Err(AppError::NotFound);
    };
    let tile: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT tile_data FROM tiles WHERE zoom_level = ? AND tile_column = ? AND tile_row = ?",
    )
    .bind(z)
    .bind(x)
    .bind(row)
    .fetch_optional(pool)
    .await?;
    let tile = tile.ok_or(AppError::NotFound)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/x-protobuf"),
            (header::CONTENT_ENCODING, "gzip"),
            (header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        tile,
    )
        .into_response())
}
