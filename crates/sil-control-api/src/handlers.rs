use crate::{ApiError, Backend, openapi, router::State};
use axum::{
    Json,
    body::Bytes,
    extract::{Path, State as AxumState},
    http::StatusCode,
};
use serde_json::{Value, json};
use sil_types::JobId;
use std::sync::Arc;

type Shared<B> = AxumState<Arc<State<B>>>;

/// Exécute un appel bloquant du nœud hors de la boucle asynchrone.
async fn blocking<B: Backend, T: Send + 'static>(
    state: &Arc<State<B>>,
    work: impl FnOnce(&B) -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    let backend = Arc::clone(&state.backend);
    tokio::task::spawn_blocking(move || work(&backend))
        .await
        .map_err(|_| ApiError::Unavailable("tâche interrompue".into()))?
}

pub(crate) async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

pub(crate) async fn openapi() -> Json<Value> {
    Json(openapi::document())
}

pub(crate) async fn put_content<B: Backend>(
    AxumState(state): Shared<B>,
    body: Bytes,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let size = body.len();
    let id = blocking(&state, move |b| b.put_content(body.to_vec())).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "content_id": id.to_hex(), "size": size })),
    ))
}

pub(crate) async fn submit_event<B: Backend>(
    AxumState(state): Shared<B>,
    body: Bytes,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    if body.is_empty() {
        return Err(ApiError::BadRequest("événement vide".into()));
    }
    let receipt = blocking(&state, move |b| b.submit_event(body.to_vec())).await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "accepted": receipt.accepted, "pending": receipt.pending })),
    ))
}

pub(crate) async fn job<B: Backend>(
    AxumState(state): Shared<B>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let job =
        JobId::from_hex(&id).map_err(|_| ApiError::BadRequest("identifiant de job".into()))?;
    blocking(&state, move |b| b.job(&job))
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

pub(crate) async fn peers<B: Backend>(
    AxumState(state): Shared<B>,
) -> Result<Json<Value>, ApiError> {
    blocking(&state, |b| b.peers()).await.map(Json)
}

pub(crate) async fn settlements<B: Backend>(
    AxumState(state): Shared<B>,
) -> Result<Json<Value>, ApiError> {
    blocking(&state, |b| b.settlements()).await.map(Json)
}

pub(crate) async fn worker<B: Backend>(
    AxumState(state): Shared<B>,
) -> Result<Json<Value>, ApiError> {
    blocking(&state, |b| b.worker()).await.map(Json)
}
