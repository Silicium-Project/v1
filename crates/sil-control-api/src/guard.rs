//! Intergiciel : authentification, scope de la route, limites de débit.

use crate::{ApiError, Principal, ROUTES, router::State};
use axum::{
    extract::{MatchedPath, Request, State as AxumState},
    middleware::Next,
    response::Response,
};
use std::{sync::Arc, time::Instant};

fn now_ms(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn lock<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// Vérifie l'accès avant le gestionnaire.
pub(crate) async fn guard<B: crate::Backend>(
    AxumState(state): AxumState<Arc<State<B>>>,
    matched: MatchedPath,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let route = ROUTES
        .iter()
        .find(|route| route.path == matched.as_str() && request_method_is(&request, route.method));
    let Some(scope) = route.and_then(|route| route.scope) else {
        return Ok(next.run(request).await);
    };
    let header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    let now = now_ms(state.started);
    let principal = state.tokens.authenticate(header);
    let Some(principal) = principal else {
        // Anti-force brute : les échecs partagent un seul seau (rafale 20, 5/s).
        let allowed = lock(&state.failures).allow(&0, 1, now);
        return Err(if allowed {
            ApiError::Unauthorized
        } else {
            ApiError::TooManyRequests
        });
    };
    if !principal.allows(scope) {
        return Err(ApiError::Forbidden);
    }
    if !lock(&state.limiter).allow(&principal.id, 1, now) {
        return Err(ApiError::TooManyRequests);
    }
    request.extensions_mut().insert::<Principal>(principal);
    Ok(next.run(request).await)
}

fn request_method_is(request: &Request, method: &str) -> bool {
    request.method().as_str().eq_ignore_ascii_case(method)
}
