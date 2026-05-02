use crate::AppState;
use crate::application::artwork as artwork_app;
use crate::domain::ArtworkId;
use crate::http::responses::binary_response;
use crate::{ApiResult, AppError};
use axum::http::StatusCode;
use axum::response::Response;

pub async fn artwork_response(state: &AppState, id: i64, size: u32) -> ApiResult<Response> {
    let (bytes, mime) = artwork_app::rendered_artwork(
        &state.repositories.artwork,
        &state.services.artwork_source_reader,
        &state.services.artwork_image_processor,
        ArtworkId::new(id),
        size,
    )
    .await
    .map_err(|err| match err.to_string().as_str() {
        "artwork source has no sidecar path"
        | "artwork source has no media path"
        | "unsupported artwork source" => AppError::not_found(err.to_string()),
        _ => AppError::from(err),
    })?;
    Ok(binary_response(StatusCode::OK, bytes, &mime, true))
}
