use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct ApiResponse<T: Serialize> {
    pub status_code: u16,
    pub status_message: String,
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Creates a successful (200 OK) response with data.
    pub fn ok(data: T) -> Self {
        Self::success(StatusCode::OK, data)
    }

    /// Creates a successful response with a specific status code and data.
    pub fn success(status: StatusCode, data: T) -> Self {
        ApiResponse {
            status_code: status.as_u16(),
            status_message: status.canonical_reason().unwrap_or("Success").to_string(),
            data: Some(data),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let body = Json(self);

        (status, body).into_response()
    }
}
