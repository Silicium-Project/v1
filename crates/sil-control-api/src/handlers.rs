use crate::{ApiError, Backend, openapi, router::State};
use axum::{
    Json,
    body::Bytes,
    extract::{Path, State as AxumState},
    http::{HeaderName, StatusCode, header},
    response::{IntoResponse, Response},
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

pub(crate) async fn submit_settlement<B: Backend>(
    AxumState(state): Shared<B>,
    body: Bytes,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    if body.is_empty() {
        return Err(ApiError::BadRequest("règlement vide".into()));
    }
    let answer = blocking(&state, move |b| b.submit_settlement(body.to_vec())).await?;
    Ok((StatusCode::ACCEPTED, Json(answer)))
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

/// Résultats bruts de tous les fragments d'un job final, concaténés (octets bruts, pour les lots).
pub(crate) async fn job_archive<B: Backend>(
    AxumState(state): Shared<B>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let job =
        JobId::from_hex(&id).map_err(|_| ApiError::BadRequest("identifiant de job".into()))?;
    let bytes = blocking(&state, move |b| b.job_archive(&job)).await?;
    Ok((
        [(header::CONTENT_TYPE, "application/octet-stream")],
        Bytes::from(bytes),
    )
        .into_response())
}

/// Résultats bruts d'un job final, fragment par fragment, en hexadécimal.
pub(crate) async fn job_outputs<B: Backend>(
    AxumState(state): Shared<B>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let job =
        JobId::from_hex(&id).map_err(|_| ApiError::BadRequest("identifiant de job".into()))?;
    let outputs = blocking(&state, move |b| b.job_outputs(&job)).await?;
    let fragments: Vec<Value> = outputs
        .into_iter()
        .map(|output| {
            let hex: String = output
                .bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            json!({ "fragment": output.fragment, "hex": hex })
        })
        .collect();
    Ok(Json(json!({ "fragments": fragments })))
}

/// Image d'un job de rendu : octets RGB24, dimensions dans `X-Image-Width` / `X-Image-Height`.
pub(crate) async fn job_image<B: Backend>(
    AxumState(state): Shared<B>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let job =
        JobId::from_hex(&id).map_err(|_| ApiError::BadRequest("identifiant de job".into()))?;
    let image = blocking(&state, move |b| b.job_image(&job)).await?;
    let width = image.width.to_string();
    let height = image.height.to_string();
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (HeaderName::from_static("x-image-width"), width),
            (HeaderName::from_static("x-image-height"), height),
        ],
        image.rgb,
    )
        .into_response())
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
