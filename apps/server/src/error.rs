use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Authentication required: {0}")]
    Unauthorized(String),

    #[error("Access forbidden: {0}")]
    Forbidden(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Validation failed: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Rate limit exceeded: try again in {0} seconds")]
    RateLimited(u32),

    #[error("Internal server error")]
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, err_type, msg) = match &self {
            ApiError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", m.clone()),
            ApiError::Forbidden(m) => (StatusCode::FORBIDDEN, "FORBIDDEN", m.clone()),
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, "NOT_FOUND", m.clone()),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", m.clone()),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, "CONFLICT", m.clone()),
            ApiError::RateLimited(secs) => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                format!("Rate limit exceeded. Retry after {} seconds.", secs),
            ),
            ApiError::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_SERVER_ERROR",
                "An unexpected internal error occurred.".to_string(),
            ),
        };

        let body = Json(ErrorResponse {
            error: err_type.to_string(),
            message: msg,
            code: None,
        });

        (status, body).into_response()
    }
}

