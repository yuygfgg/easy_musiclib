use crate::ApiResult;
use crate::http::range::{RequestedRange, requested_range};
use axum::body::Body;
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

pub async fn ranged_file_response(
    path: &Path,
    mime: &str,
    download_name: Option<(&str, &str)>,
    headers: &HeaderMap,
    download: bool,
) -> ApiResult<Response> {
    let mut file = tokio::fs::File::open(path).await?;
    let len = file.metadata().await?.len();
    let (status, start, end) = match requested_range(headers, len) {
        RequestedRange::None => (StatusCode::OK, 0, len.saturating_sub(1)),
        RequestedRange::Valid(start, end) => (StatusCode::PARTIAL_CONTENT, start, end),
        RequestedRange::Invalid => return Ok(range_not_satisfiable_response(len, Some(mime))),
    };
    let read_len = if len == 0 {
        0
    } else {
        end.saturating_sub(start).saturating_add(1)
    };
    file.seek(std::io::SeekFrom::Start(start)).await?;
    let stream = ReaderStream::new(file.take(read_len));
    let mut response = (status, Body::from_stream(stream)).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_str(mime).unwrap());
    response.headers_mut().insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&read_len.to_string()).unwrap(),
    );
    response
        .headers_mut()
        .insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if status == StatusCode::PARTIAL_CONTENT {
        response.headers_mut().insert(
            CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{len}")).unwrap(),
        );
    }
    if download {
        let Some((title, extension)) = download_name else {
            return Ok(response);
        };
        let suffix = if extension.is_empty() {
            String::new()
        } else {
            format!(".{extension}")
        };
        response.headers_mut().insert(
            CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!(
                "attachment; filename*=UTF-8''{}{}",
                percent_encoding::utf8_percent_encode(title, percent_encoding::NON_ALPHANUMERIC),
                suffix
            ))
            .unwrap(),
        );
    }
    Ok(response)
}

pub fn audio_bytes_response(
    bytes: Vec<u8>,
    mime: &str,
    extension: &str,
    title: &str,
    download: bool,
    headers: &HeaderMap,
) -> Response {
    let len = bytes.len() as u64;
    let (status, start, end) = if download {
        (StatusCode::OK, 0, len.saturating_sub(1))
    } else {
        match requested_range(headers, len) {
            RequestedRange::None => (StatusCode::OK, 0, len.saturating_sub(1)),
            RequestedRange::Valid(start, end) => (StatusCode::PARTIAL_CONTENT, start, end),
            RequestedRange::Invalid => return range_not_satisfiable_response(len, Some(mime)),
        }
    };
    let body = if len == 0 {
        bytes
    } else {
        bytes[start as usize..=end as usize].to_vec()
    };
    let read_len = body.len();
    let mut response = (status, Body::from(body)).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_str(mime).unwrap());
    response.headers_mut().insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&read_len.to_string()).unwrap(),
    );
    response
        .headers_mut()
        .insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    if status == StatusCode::PARTIAL_CONTENT {
        response.headers_mut().insert(
            CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{len}")).unwrap(),
        );
    }
    if download {
        response.headers_mut().insert(
            CONTENT_DISPOSITION,
            HeaderValue::from_str(&format!(
                "attachment; filename*=UTF-8''{}.{}",
                percent_encoding::utf8_percent_encode(title, percent_encoding::NON_ALPHANUMERIC),
                extension,
            ))
            .unwrap(),
        );
    }
    response
}

pub fn binary_response(status: StatusCode, bytes: Vec<u8>, mime: &str, cache: bool) -> Response {
    let len = bytes.len();
    let mut response = (status, Body::from(bytes)).into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_str(mime).unwrap());
    response.headers_mut().insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&len.to_string()).unwrap(),
    );
    if cache {
        response.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    response
}

pub fn range_not_satisfiable_response(len: u64, mime: Option<&str>) -> Response {
    let mut response = (
        StatusCode::RANGE_NOT_SATISFIABLE,
        Body::from(Vec::<u8>::new()),
    )
        .into_response();
    response
        .headers_mut()
        .insert(CONTENT_LENGTH, HeaderValue::from_static("0"));
    response
        .headers_mut()
        .insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    response.headers_mut().insert(
        CONTENT_RANGE,
        HeaderValue::from_str(&format!("bytes */{len}")).unwrap(),
    );
    if let Some(mime) = mime {
        response
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_str(mime).unwrap());
    }
    response
}
