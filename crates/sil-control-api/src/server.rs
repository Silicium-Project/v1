use axum::Router;
use std::future::Future;
use tokio::net::TcpListener;

/// Sert l'API jusqu'à ce que `shutdown` se termine, en finissant les requêtes en cours.
pub async fn serve(
    listener: TcpListener,
    router: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
}
