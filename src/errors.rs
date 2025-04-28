use crate::response::ApiResponse;
use anyhow::anyhow;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use deadpool_diesel::InteractError;
use deadpool_diesel::postgres::PoolError;
use thiserror::Error;
use tracing::error;

#[derive(Debug, Error)]
pub enum AppError {
    #[allow(dead_code)]
    #[error("Bad Request: {0}")]
    BadRequest(String), // 400

    #[allow(dead_code)]
    #[error("Unauthorized: {0}")]
    Unauthorized(String), // 401

    #[error("Forbidden: {0}")]
    Forbidden(String), // 403

    #[error("Not Found: {0}")]
    NotFound(String), // 404

    #[error("Conflict: {0}")]
    Conflict(String), // 409

    #[error("Unprocessable Entity: {0}")]
    UnprocessableEntity(String), // 422

    #[error("Internal Server Error: {0}")]
    InternalServerError(#[from] anyhow::Error), // 500
}

impl From<PoolError> for AppError {
    fn from(err: PoolError) -> Self {
        error!("Database pool error encountered: {:?}", err);
        AppError::InternalServerError(anyhow::Error::new(err).context("Database pool error"))
    }
}

impl From<InteractError> for AppError {
    fn from(err: InteractError) -> Self {
        error!("Database interaction error encountered: {:?}", err);
        AppError::InternalServerError(anyhow!("Database interaction error: {}", err))
    }
}

impl From<diesel::result::Error> for AppError {
    fn from(err: diesel::result::Error) -> Self {
        match err {
            diesel::result::Error::NotFound => {
                error!(
                    "Diesel NotFound error reached generic conversion: {:?}",
                    err
                );
                AppError::NotFound("Resource not found (database query)".to_string())
            }
            _ => {
                error!("Unhandled Diesel error encountered: {:?}", err);
                AppError::InternalServerError(
                    anyhow::Error::new(err).context("Database query error"),
                )
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            AppError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message),
            AppError::Forbidden(message) => (StatusCode::FORBIDDEN, message),
            AppError::NotFound(message) => (StatusCode::NOT_FOUND, message),
            AppError::Conflict(message) => (StatusCode::CONFLICT, message),
            AppError::UnprocessableEntity(message) => (StatusCode::UNPROCESSABLE_ENTITY, message),

            AppError::InternalServerError(source) => {
                error!(
                    "Responding with 500 Internal Server Error. Source: {:?}",
                    source
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal server error occurred".to_string(),
                )
            }
        };

        let body = ApiResponse::<()> {
            status_code: status.as_u16(),
            status_message: error_message,
            data: None,
        };

        (status, body).into_response()
    }
}
