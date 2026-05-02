use crate::application::artwork::{ArtworkImageProcessor, ArtworkSourceReader};
use anyhow::Result;
use easy_musiclib_media::metadata::read_embedded_picture_for_path;
use futures::FutureExt;
use futures::future::BoxFuture;
use std::io::Cursor;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct FilesystemArtworkSourceReader;

impl ArtworkSourceReader for FilesystemArtworkSourceReader {
    fn read_sidecar_artwork<'a>(
        &'a self,
        path: &'a str,
    ) -> BoxFuture<'a, Result<(Vec<u8>, String)>> {
        async move {
            let bytes = tokio::fs::read(path).await?;
            let mime = mime_guess::from_path(path)
                .first_raw()
                .unwrap_or("application/octet-stream")
                .to_string();
            Ok((bytes, mime))
        }
        .boxed()
    }

    fn read_embedded_artwork<'a>(
        &'a self,
        path: &'a str,
        picture_index: i64,
    ) -> BoxFuture<'a, Result<(Vec<u8>, String)>> {
        async move {
            let path = path.to_string();
            tokio::task::spawn_blocking(move || {
                read_embedded_picture_for_path(Path::new(&path), picture_index)
            })
            .await?
            .map(|(bytes, mime)| (bytes, mime.unwrap_or_else(|| "image/jpeg".to_string())))
        }
        .boxed()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImageArtworkProcessor;

impl ArtworkImageProcessor for ImageArtworkProcessor {
    fn resize_jpeg(&self, bytes: Vec<u8>, size: u32) -> BoxFuture<'_, Result<Vec<u8>>> {
        async move { tokio::task::spawn_blocking(move || resize_image(bytes, size)).await? }.boxed()
    }
}

fn resize_image(bytes: Vec<u8>, size: u32) -> Result<Vec<u8>> {
    let image = image::load_from_memory(&bytes)?;
    let resized = image.thumbnail(size, size);
    let mut out = Cursor::new(Vec::new());
    resized.write_to(&mut out, image::ImageFormat::Jpeg)?;
    Ok(out.into_inner())
}
