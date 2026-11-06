use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

/// Erreur renvoyée au client. Les messages ne contiennent jamais de chemin ni de secret.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ApiError {
    #[error("requête invalide : {0}")]
    BadRequest(String),
    #[error("introuvable")]
    NotFound,
    #[error("corps trop grand")]
    TooLarge,
    #[error("non authentifié")]
    Unauthorized,
    #[error("droits insuffisants")]
    Forbidden,
    #[error("trop de requêtes")]
    TooManyRequests,
    #[error("service indisponible : {0}")]
    Unavailable(String),
}

impl ApiError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Self::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(json!({ "error": self.to_string() }));
        (self.status(), body).into_response()
    }
}
