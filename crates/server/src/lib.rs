pub mod app;
pub mod db;
pub mod handlers;
pub mod import_old;
pub mod lyrics;
pub mod scanner;
pub mod schema;

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use easy_musiclib_shared::ApiError;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub static_dir: Arc<std::path::PathBuf>,
}

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        tracing::error!(error = %value, "request failed");
        Self::internal(value.to_string())
    }
}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        tracing::error!(error = %value, "database request failed");
        Self::internal(value.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        tracing::error!(error = %value, "io request failed");
        Self::internal(value.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ApiError {
                message: self.message,
            }),
        )
            .into_response()
    }
}

pub type ApiResult<T> = Result<T, AppError>;
