use super::support::*;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use sil_types::JobId;
use tower::ServiceExt;

#[tokio::test]
async fn the_input_of_a_job_comes_as_raw_bytes_to_a_reader() {
    let (app, _) = app();
    let request = Request::builder()
        .method("GET")
        .uri(format!(
            "/v1/jobs/{}/input",
            JobId::from_bytes([1; 32]).to_hex()
        ))
        .header("authorization", format!("Bearer {}", secret(READER)))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "application/octet-stream"
    );
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[..], b"un\ndeux");
}

#[tokio::test]
async fn the_input_needs_a_token_and_an_unknown_job_is_not_found() {
    let (app, _) = app();
    let unauthenticated = Request::builder()
        .method("GET")
        .uri(format!(
            "/v1/jobs/{}/input",
            JobId::from_bytes([1; 32]).to_hex()
        ))
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(unauthenticated).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    let unknown = Request::builder()
        .method("GET")
        .uri(format!(
            "/v1/jobs/{}/input",
            JobId::from_bytes([9; 32]).to_hex()
        ))
        .header("authorization", format!("Bearer {}", secret(READER)))
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(unknown).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
}
