use crate::{Backend, TokenSet, guard::guard, handlers};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{get, post, put},
};
use sil_quota::KeyedLimiter;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

/// Bornes de l'API.
#[derive(Debug, Clone)]
pub struct ApiLimits {
    /// Taille maximale d'un contenu envoyé.
    pub max_upload_bytes: usize,
    /// Taille maximale d'un événement signé.
    pub max_event_bytes: usize,
    /// Requêtes autorisées par jeton : rafale et débit par seconde.
    pub burst: u32,
    pub per_sec: u32,
}

impl Default for ApiLimits {
    fn default() -> Self {
        Self {
            max_upload_bytes: 64 * 1024 * 1024,
            max_event_bytes: 64 * 1024,
            burst: 60,
            per_sec: 20,
        }
    }
}

pub(crate) struct State<B> {
    pub(crate) backend: Arc<B>,
    pub(crate) tokens: TokenSet,
    pub(crate) limiter: Mutex<KeyedLimiter<usize>>,
    pub(crate) failures: Mutex<KeyedLimiter<u8>>,
    pub(crate) started: Instant,
}

/// Construit le routeur. Toutes les routes protégées passent par `guard`.
pub fn router<B: Backend>(backend: Arc<B>, tokens: TokenSet, limits: ApiLimits) -> Router {
    let state = Arc::new(State {
        backend,
        limiter: Mutex::new(KeyedLimiter::new(1024, limits.burst, limits.per_sec)),
        failures: Mutex::new(KeyedLimiter::new(1, 20, 5)),
        tokens,
        started: Instant::now(),
    });
    Router::new()
        .route("/health", get(handlers::health))
        .route("/v1/openapi.json", get(handlers::openapi))
        .route(
            "/v1/content",
            put(handlers::put_content::<B>).layer(DefaultBodyLimit::max(limits.max_upload_bytes)),
        )
        .route(
            "/v1/events",
            post(handlers::submit_event::<B>).layer(DefaultBodyLimit::max(limits.max_event_bytes)),
        )
        .route("/v1/jobs/{id}", get(handlers::job::<B>))
        .route("/v1/jobs/{id}/image", get(handlers::job_image::<B>))
        .route("/v1/jobs/{id}/outputs", get(handlers::job_outputs::<B>))
        .route("/v1/jobs/{id}/archive", get(handlers::job_archive::<B>))
        .route("/v1/peers", get(handlers::peers::<B>))
        .route(
            "/v1/settlements",
            get(handlers::settlements::<B>)
                .post(handlers::submit_settlement::<B>)
                .layer(DefaultBodyLimit::max(limits.max_event_bytes)),
        )
        .route("/v1/worker", get(handlers::worker::<B>))
        .route_layer(middleware::from_fn_with_state(state.clone(), guard::<B>))
        .with_state(state)
}
