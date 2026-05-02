use crate::http::responses::binary_response;
use crate::{ApiResult, AppError, AppState, db};
use axum::http::StatusCode;
use axum::response::Response;
use easy_musiclib_media::metadata::read_embedded_picture_for_path;
use std::io::Cursor;

pub async fn artwork_response(state: &AppState, id: i64, size: u32) -> ApiResult<Response> {
    let size = size.clamp(32, 2000);
    let variant = format!("size={size}");
    if let Some((bytes, mime)) = db::get_artwork_blob(&state.pool, id, &variant).await? {
        return Ok(binary_response(StatusCode::OK, bytes, &mime, true));
    }
    let source = db::source_for_artwork(&state.pool, id).await?;
    let (raw, _mime) = match source.kind.as_str() {
        "sidecar" => {
            let path = source
                .sidecar_path
                .ok_or_else(|| AppError::not_found("artwork source has no sidecar path"))?;
            let bytes = tokio::fs::read(&path).await?;
            let mime = mime_guess::from_path(&path)
                .first_raw()
                .unwrap_or("application/octet-stream")
                .to_string();
            (bytes, mime)
        }
        "embedded" => {
            let path = source
                .media_path
                .ok_or_else(|| AppError::not_found("artwork source has no media path"))?;
            tokio::task::spawn_blocking(move || {
                read_embedded_picture_for_path(
                    std::path::Path::new(&path),
                    source.embedded_picture_index.unwrap_or(0),
                )
            })
            .await
            .map_err(|e| AppError::internal(e.to_string()))?
            .map(|(bytes, mime)| (bytes, mime.unwrap_or_else(|| "image/jpeg".to_string())))?
        }
        _ => return Err(AppError::not_found("unsupported artwork source")),
    };
    let resized = tokio::task::spawn_blocking(move || resize_image(raw, size))
        .await
        .map_err(|e| AppError::internal(e.to_string()))??;
    db::put_artwork_blob(
        &state.pool,
        id,
        &variant,
        "image/jpeg",
        Some(size as i64),
        None,
        resized.clone(),
    )
    .await
    .ok();
    Ok(binary_response(StatusCode::OK, resized, "image/jpeg", true))
}

fn resize_image(bytes: Vec<u8>, size: u32) -> anyhow::Result<Vec<u8>> {
    let image = image::load_from_memory(&bytes)?;
    let resized = image.thumbnail(size, size);
    let mut out = Cursor::new(Vec::new());
    resized.write_to(&mut out, image::ImageFormat::Jpeg)?;
    Ok(out.into_inner())
}
