use core::fmt;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;

use super::dto::Message;

#[derive(Debug)]
pub struct Error {
    pub code: StatusCode,
    pub body: Json<Message>,
}

impl Error {
    pub fn new(code: StatusCode, message: &str) -> Self {
        Self {
            code,
            body: Json(Message::new(message)),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (self.code, self.body).into_response()
    }
}

impl From<(StatusCode, &str)> for Error {
    fn from((code, msg): (StatusCode, &str)) -> Self {
        Self::new(code, msg)
    }
}

impl From<sqlx::error::Error> for Error {
    fn from(error: sqlx::error::Error) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string())
    }
}

impl From<jsonwebtoken::errors::Error> for Error {
    fn from(error: jsonwebtoken::errors::Error) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string())
    }
}

impl From<argon2::password_hash::errors::Error> for Error {
    fn from(error: argon2::password_hash::errors::Error) -> Self {
        Self::new(StatusCode::BAD_REQUEST, &error.to_string())
    }
}

#[derive(Debug)]
pub enum TokenHolderError {
    ReqwestError(reqwest::Error),
    JsonError(serde_json::Error),
    ApiError(String),
}

impl fmt::Display for TokenHolderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenHolderError::ReqwestError(e) => write!(f, "Reqwest error: {}", e),
            TokenHolderError::JsonError(e) => write!(f, "JSON error: {}", e),
            TokenHolderError::ApiError(e) => write!(f, "API error: {}", e),
        }
    }
}

impl std::error::Error for TokenHolderError {}

impl From<reqwest::Error> for TokenHolderError {
    fn from(error: reqwest::Error) -> Self {
        TokenHolderError::ReqwestError(error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        // You might want to use a specific status code based on the error type
        let status_code = match error.status() {
            Some(status) => status,
            None => StatusCode::INTERNAL_SERVER_ERROR, // Default status code
        };
        Self::new(status_code, &error.to_string())
    }
}

impl From<serde_json::Error> for TokenHolderError {
    fn from(error: serde_json::Error) -> Self {
        TokenHolderError::JsonError(error)
    }
}
