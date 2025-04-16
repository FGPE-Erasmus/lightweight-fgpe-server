use crate::response::ApiResponse;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use deadpool_diesel::InteractError;
use deadpool_diesel::postgres::PoolError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database pool error")]
    PoolError(#[from] PoolError),

    #[error("Database interaction error")]
    InteractError(#[from] InteractError),

    #[error("Database query failed: {0}")]
    DieselError(#[from] diesel::result::Error),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::NotFound(message) => (StatusCode::NOT_FOUND, message),
            AppError::PoolError(_)
            | AppError::InteractError(_)
            | AppError::DieselError(_)
            | AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "An internal server error occurred".to_string(),
            ),
        };

        let body = ApiResponse::<()> {
            status_code: status.as_u16(),
            status_message: error_message,
            data: None,
        };

        (status, body).into_response()
    }
}
